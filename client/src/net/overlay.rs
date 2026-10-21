use crossbeam_channel::{Receiver, Sender};
use macroquad::{
    prelude::*,
    ui::{
        hash, root_ui,
        widgets::{Editbox, Window},
    },
};
use rhythm_game_server::protocol::User;
use triple_buffer::{Input, Output};

use crate::{
    AssetStore,
    net::{NetRequest, NetResponse},
};

#[derive(Debug, Clone, PartialEq)]
enum ConnectionState {
    Connected,
    Disconnected,
    Waiting,
}

pub struct NetOverlay {
    request: u32,
    ping: u32,
    connected: ConnectionState,
    user: Option<User>,
    input: Input<Option<NetOverlayRenderData>>,
    req_tx: Sender<(u32, NetRequest)>,
    res_rx: Receiver<(u32, NetResponse)>,
    ui_tx: Sender<UiEvent>,
    ui_rx: Receiver<UiEvent>,
    show_window: bool,
    username_entry: String,
    password_entry: String,
    status_text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NetOverlayRenderData {
    ping: u32,
    connected: ConnectionState,
    show_window: bool,
    user: Option<User>,
    username_entry: String,
    password_entry: String,
    login_status_text: Option<String>,
    ui_tx: Sender<UiEvent>,
}

enum UiEvent {
    ShowWindow,
    HideWindow,
    SetUsername(String),
    SetPassword(String),
    Login,
    Register,
}

impl NetOverlay {
    pub fn new(
        req_tx: Sender<(u32, NetRequest)>,
        res_rx: Receiver<(u32, NetResponse)>,
        input: Input<Option<NetOverlayRenderData>>,
    ) -> Self {
        let (ui_tx, ui_rx) = crossbeam_channel::unbounded();

        // perform initial status check

        let mut request = 1u32;
        if let Err(why) = req_tx.send((request, NetRequest::ConnectionStatus)) {
            warn!(
                "failed to perform initial status check {:?}, using offline mode",
                why
            );
            return Self {
                request,
                ping: 0,
                connected: ConnectionState::Disconnected,
                user: None,
                req_tx,
                res_rx,
                ui_tx,
                ui_rx,
                input,
                show_window: false,
                username_entry: "".to_string(),
                password_entry: "".to_string(),
                status_text: None,
            };
        }
        request += 1;

        Self {
            request,
            ping: 0,
            connected: ConnectionState::Waiting,
            user: None,
            req_tx,
            res_rx,
            ui_tx,
            ui_rx,
            input,
            show_window: true,
            username_entry: "".to_string(),
            password_entry: "".to_string(),
            status_text: None,
        }
    }

    /// login callback
    fn login(&self) {}

    /// register callback
    fn register(&mut self) {
        if self.connected != ConnectionState::Connected {
            self.status_text = Some("Online services cannot be reached".to_string());
        }

        // perform input validation
        if self.username_entry.is_empty() || self.password_entry.is_empty() {
            self.status_text = Some("Username and password cannot be empty".to_string());
            return;
        }
        if self.username_entry.len() < 3 || self.username_entry.len() > 32 {
            self.status_text = Some("Username must be between 3 and 32 characters".to_string());
            return;
        }
        if self.password_entry.len() < 8 || self.password_entry.len() > 64 {
            self.status_text = Some("Password must be between 8 and 64 characters".to_string());
            return;
        }

        // send message to net thread
        if let Err(why) = self.req_tx.send((
            self.request,
            NetRequest::UserRegister {
                username: self.username_entry.clone(),
                password: self.password_entry.clone(),
            },
        )) {
            error!("failed to send request to net thread: {:?}", why);
            self.status_text = Some("Failed to send request to net thread".to_string());
            self.request += 1;
            return;
        }

        self.request += 1;
        self.status_text = Some("Registering...".to_string());
    }

    pub fn update(&mut self) {
        self.res_rx
            .try_iter()
            .for_each(|(_id, response)| match response {
                NetResponse::ConnectionStatus { connected, ping } => {
                    self.ping = ping;
                    self.connected = if connected {
                        ConnectionState::Connected
                    } else {
                        ConnectionState::Disconnected
                    };
                }
                NetResponse::LoginStatus { user, token: _ } => {
                    self.status_text = if let Some(user) = user {
                        self.user = Some(user.clone());
                        Some(format!("Logged in as {}", user.name))
                    } else {
                        Some("Not logged in".to_string())
                    };
                }
                _ => {}
            });

        // process UI events
        while let Ok(event) = self.ui_rx.try_recv() {
            match event {
                UiEvent::ShowWindow => self.show_window = true,
                UiEvent::HideWindow => self.show_window = false,
                UiEvent::SetUsername(username) => self.username_entry = username,
                UiEvent::SetPassword(password) => self.password_entry = password,
                UiEvent::Login => {
                    self.status_text = Some("Logging in...".to_string());
                }
                UiEvent::Register => {
                    self.register();
                }
            }
        }

        self.input.write(Some(NetOverlayRenderData {
            ping: self.ping,
            connected: self.connected.clone(),
            show_window: self.show_window,
            username_entry: self.username_entry.clone(),
            password_entry: self.password_entry.clone(),
            login_status_text: self.status_text.clone(),
            ui_tx: self.ui_tx.clone(),
            user: self.user.clone(),
        }));
    }
}

fn draw_window(data: &NetOverlayRenderData) {
    let (w, h) = (screen_width(), screen_height());
    let mut ui = root_ui();
    let mut username = data.username_entry.clone();
    let mut password = data.password_entry.clone();
    let mut log_in = false;
    let mut register = false;

    let opened = Window::new(
        hash!("login-ui"),
        vec2(w * 0.3, h * 0.2),
        vec2(w * 0.4, h * 0.55),
    )
    .label("Login")
    .close_button(true)
    .ui(&mut ui, |ui| {
        ui.label(None, "Username");
        Editbox::new(hash!("username"), vec2(300., 30.))
            .multiline(false)
            .ui(ui, &mut username);
        ui.label(None, "Password");
        Editbox::new(hash!("password"), vec2(300., 30.))
            .multiline(false)
            .ui(ui, &mut password);
        log_in = ui.button(None, "Log in");
        register = ui.button(None, "Register");

        if let Some(status) = &data.login_status_text {
            ui.label(None, status);
        }
    });

    if !opened {
        data.ui_tx.send(UiEvent::HideWindow).unwrap();
    }

    if username != data.username_entry {
        data.ui_tx
            .send(UiEvent::SetUsername(username.clone()))
            .unwrap();
    }

    if password != data.password_entry {
        data.ui_tx
            .send(UiEvent::SetPassword(password.clone()))
            .unwrap();
    }

    if log_in {
        data.ui_tx.send(UiEvent::Login).unwrap();
    }

    if register {
        data.ui_tx.send(UiEvent::Register).unwrap();
    }
}

pub fn draw(data: &NetOverlayRenderData, assets: &AssetStore) {
    let ping = data.ping;
    let connected = format!("ONLINE ({}ms)", ping);

    set_default_camera();

    // draw window
    if data.show_window {
        draw_window(&data);
    }

    match data.connected {
        ConnectionState::Connected => draw_texture(
            &assets.load().globe_icon,
            20.0,
            screen_height() - 40.0,
            GREEN,
        ),
        ConnectionState::Disconnected => draw_texture(
            &assets.load().globe_off_icon,
            20.0,
            screen_height() - 40.0,
            GREEN,
        ),
        _ => {}
    }
    draw_text(
        match data.connected {
            ConnectionState::Connected => &connected,
            ConnectionState::Disconnected => "OFFLINE",
            ConnectionState::Waiting => "Waiting for server...",
        },
        58.0,
        screen_height() - 22.0,
        24.0,
        BLACK,
    );

    draw_text(
        if let Some(user) = &data.user {
            format!("Logged in as {}", user.name)
        } else {
            "Not logged in".to_string()
        },
        20.0,
        screen_height() - 60.0,
        24.0,
        BLACK,
    );
}
