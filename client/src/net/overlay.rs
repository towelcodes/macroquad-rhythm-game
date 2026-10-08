use crossbeam_channel::{Receiver, Sender};
use macroquad::prelude::*;
use triple_buffer::{Input, Output};

use crate::{
    AssetStore,
    net::{NetRequest, NetResponse},
};

#[derive(Debug, Clone)]
enum ConnectionState {
    Connected,
    Disconnected,
    Waiting,
}

pub struct NetOverlay {
    request: u32,
    ping: u32,
    connected: ConnectionState,
    input: Input<NetOverlayRenderData>,
    req_tx: Sender<(u32, NetRequest)>,
    res_rx: Receiver<(u32, NetResponse)>,
}

#[derive(Debug, Clone)]
pub struct NetOverlayRenderData {
    ping: u32,
    connected: ConnectionState,
}
impl Default for NetOverlayRenderData {
    fn default() -> Self {
        Self {
            ping: 0,
            connected: ConnectionState::Waiting,
        }
    }
}

impl NetOverlay {
    pub fn new(
        req_tx: Sender<(u32, NetRequest)>,
        res_rx: Receiver<(u32, NetResponse)>,
        input: Input<NetOverlayRenderData>,
    ) -> Self {
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
                req_tx,
                res_rx,
                input,
            };
        }
        request += 1;

        Self {
            request,
            ping: 0,
            connected: ConnectionState::Waiting,
            req_tx,
            res_rx,
            input,
        }
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
                _ => {}
            });
        self.input.write(NetOverlayRenderData {
            ping: self.ping,
            connected: self.connected.clone(),
        });
    }
}

pub fn draw(data: &NetOverlayRenderData, assets: &AssetStore) {
    let ping = data.ping;
    let connected = format!("ONLINE ({}ms)", ping);

    set_default_camera();
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
}
