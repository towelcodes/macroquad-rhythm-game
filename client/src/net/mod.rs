use std::{
    io::ErrorKind,
    net::TcpStream,
    sync::TryLockError::WouldBlock,
    time::{Duration, Instant},
};

use crate::{
    data::GameConfig,
    net::overlay::NetOverlayRenderData,
    util::ui::{self, AnchorPoint},
};
use crossbeam_channel::{Receiver, Sender};
use macroquad::prelude::*;
use rhythm_game_server::protocol::{ClientboundPacket, User, recv, send};
use triple_buffer::Input;

pub mod overlay;

struct OnlineUser {
    active_token: [u8; 32],
    user: User,
}

#[derive(Debug, Clone)]
pub enum NetRequest {
    Status,
    UserRegister { username: String, password: String },
    UserLogin { username: String, password: String },
}

#[derive(Debug, Clone)]
pub enum NetResponse {
    Ok,
    Error,
    Disconnected,
    Timeout,
    Status {
        user: Option<User>,
        connected: bool,
        ping: u32,
    },
}

const BACKEND_ADDRESS: &str = "127.0.0.1:7300";
pub fn start_net_thread(req_rx: Receiver<(u32, NetRequest)>, res_tx: Sender<(u32, NetResponse)>) {
    let mut active_status_request: Option<(u32, Instant)> = None;

    // create connection
    match TcpStream::connect(BACKEND_ADDRESS) {
        Ok(mut stream) => {
            info!("connected to backend! {:?}", stream);

            // send ping
            if let Err(why) = send(ClientboundPacket::Ping, &mut stream) {
                warn!("error sending ping: {:?}", why);
            }
            stream
                .set_read_timeout(Some(Duration::from_millis(500)))
                .expect("failed to set read timeout");

            loop {
                match recv::<ClientboundPacket>(&mut stream) {
                    Ok(packet) => {
                        info!("received packet: {:?}", packet);
                        match packet {
                            ClientboundPacket::Pong => {
                                info!("received pong from server");
                                if let Some(status_request) = active_status_request {
                                    if let Err(why) = res_tx.send((
                                        status_request.0,
                                        NetResponse::Status {
                                            user: None,
                                            connected: true,
                                            ping: status_request.1.elapsed().as_millis() as u32,
                                        },
                                    )) {
                                        warn!("error sending net response: {:?}", why);
                                    }
                                }
                            }
                            ClientboundPacket::Ping => {
                                info!("received ping from server, sending pong");
                                if let Err(why) = send(ClientboundPacket::Pong, &mut stream) {
                                    warn!("error sending pong: {:?}", why);
                                }
                            }
                            _ => {
                                info!("packet not implemented");
                            }
                        }
                    }
                    Err(e) => match e.kind() {
                        ErrorKind::WouldBlock => {}
                        _ => {
                            warn!("error receiving packet: {:?}", e);
                            break;
                        }
                    },
                }

                req_rx.try_iter().for_each(|(id, req)| {
                    info!("received net request {}: {:?}", id, req);
                    match req {
                        NetRequest::Status => {
                            // send status request
                            if let Err(why) = send(ClientboundPacket::Ping, &mut stream) {
                                warn!("error sending ping: {:?}", why);
                                res_tx
                                    .send((id, NetResponse::Error))
                                    .expect("failed to send net response");
                            } else {
                                active_status_request = Some((id, Instant::now()));
                            }
                        }
                        _ => {
                            warn!("not implemented");
                        }
                    }
                });
            }
        }
        Err(why) => {
            warn!("could not reach online services: {:?}", why);
        }
    }
}

struct NetState {
    active_user: Option<OnlineUser>,
}

impl NetState {
    /// Draws an overlay with the connection information
    /// Must be called from the main thread
    pub fn draw_net_overlay(&self) {
        ui::label((vec2(0., 1.), AnchorPoint::BottomLeft), "LOCAL MODE");
    }
}
