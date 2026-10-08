use crossbeam_channel::{Receiver, Sender};
use macroquad::{camera::set_default_camera, logging::warn};
use triple_buffer::{Input, Output};

use crate::{
    net::{NetRequest, NetResponse},
    util::ui,
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
        if let Err(why) = req_tx.send((request, NetRequest::Status)) {
            warn!("failed to perform initial status check {:?}", why);
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
                NetResponse::Status {
                    user: _,
                    connected,
                    ping,
                } => {
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

pub fn draw(data: &NetOverlayRenderData) {
    let ping = data.ping;
    let connected = format!("ONLINE ({}ms)", ping);
    ui::label(
        None,
        match data.connected {
            ConnectionState::Connected => &connected,
            ConnectionState::Disconnected => "OFFLINE",
            ConnectionState::Waiting => "Server is busy...",
        },
    );
}
