use std::{
    fs::{File, read_to_string},
    io::{self, ErrorKind, Write},
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
use rhythm_game_server::protocol::{ClientboundPacket, ServerboundPacket, User, recv, send};
use serde::{Deserialize, Serialize};
use triple_buffer::Input;

pub mod overlay;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OnlineUser {
    pub active_token: [u8; 32],
    pub user: User,
}

#[derive(Debug, Clone)]
pub enum NetRequest {
    ConnectionStatus,
    LoginStatus,
    UserRegister { username: String, password: String },
    UserLogin { username: String, password: String },
}

#[derive(Debug, Clone)]
pub enum NetResponse {
    Ok,
    Error,
    Disconnected,
    Timeout,
    ConnectionStatus { connected: bool, ping: u32 },
    LoginStatus(Option<OnlineUser>),
}

fn load_user_data(path: &str) -> Option<OnlineUser> {
    match read_to_string(path) {
        Ok(data) => ron::from_str(&data).unwrap_or(None),
        Err(_) => None,
    }
}

fn save_user_data(path: &str, data: Option<OnlineUser>) -> Result<(), io::Error> {
    // save user login data
    let mut output = File::create(path)?;
    let ron = ron::to_string(&data).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    write!(output, "{}", ron)?;
    Ok(())
}

const BACKEND_ADDRESS: &str = "127.0.0.1:7300";
const USER_PATH: &str = "user.ron";
pub fn start_net_thread(req_rx: Receiver<(u32, NetRequest)>, res_tx: Sender<(u32, NetResponse)>) {
    let mut active_status_request: Option<(u32, Instant)> = None;

    // load user data file
    let mut active_online_user: Option<OnlineUser> = load_user_data(USER_PATH);
    info!("loaded user login state: {:?}", active_online_user);

    // create connection
    match TcpStream::connect(BACKEND_ADDRESS) {
        Ok(mut stream) => {
            info!("connected to backend! {:?}", stream);

            // send ping
            if let Err(why) = send(ClientboundPacket::Ping(0), &mut stream) {
                warn!("error sending ping: {:?}", why);
            }
            stream
                .set_read_timeout(Some(Duration::from_millis(500)))
                .expect("failed to set read timeout");

            loop {
                // read incoming packets
                match recv::<ClientboundPacket>(&mut stream) {
                    Ok(packet) => {
                        info!("received packet: {:?}", packet);
                        match packet {
                            ClientboundPacket::Pong(id) => {
                                info!("received pong from server");
                                if let Some(status_request) = active_status_request {
                                    if let Err(why) = res_tx.send((
                                        status_request.0,
                                        NetResponse::ConnectionStatus {
                                            connected: true,
                                            ping: status_request.1.elapsed().as_millis() as u32,
                                        },
                                    )) {
                                        warn!("error sending net response: {:?}", why);
                                    }
                                }
                            }
                            ClientboundPacket::Ping(id) => {
                                info!("received ping from server, sending pong");
                                if let Err(why) = send(ClientboundPacket::Pong(id), &mut stream) {
                                    warn!("error sending pong: {:?}", why);
                                }
                            }
                            ClientboundPacket::LoggedIn {
                                id,
                                uid,
                                username,
                                token,
                            } => {
                                info!("Logged in {} {}", id, uid);

                                active_online_user = Some(OnlineUser {
                                    active_token: token,
                                    user: User {
                                        name: username.clone(),
                                        id: uid,
                                    },
                                });

                                if let Err(why) = res_tx.send((
                                    id,
                                    NetResponse::LoginStatus(active_online_user.clone()),
                                )) {
                                    warn!("error sending net response: {:?}", why);
                                }

                                if let Err(why) =
                                    save_user_data(USER_PATH, active_online_user.clone())
                                {
                                    warn!("error saving user data: {:?}", why);
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
                            res_tx
                                .send((
                                    0,
                                    NetResponse::ConnectionStatus {
                                        connected: false,
                                        ping: 0,
                                    },
                                ))
                                .expect("failed to send net response");
                            break;
                        }
                    },
                }

                // read requests
                req_rx.try_iter().for_each(|(id, req)| {
                    info!("received net request {}: {:?}", id, req);
                    match req {
                        NetRequest::LoginStatus => {
                            // reply with the current login status
                            res_tx
                                .send((id, NetResponse::LoginStatus(active_online_user.clone())))
                                .expect("failed to send net response");
                        }
                        NetRequest::ConnectionStatus => {
                            // send status request
                            if let Err(why) = send(ServerboundPacket::Ping(0), &mut stream) {
                                warn!("error sending ping: {:?}", why);
                                res_tx
                                    .send((id, NetResponse::Error))
                                    .expect("failed to send net response");
                            } else {
                                active_status_request = Some((id, Instant::now()));
                            }
                        }
                        NetRequest::UserRegister { username, password } => {
                            if let Err(why) = send(
                                ServerboundPacket::Register {
                                    id,
                                    username,
                                    password,
                                },
                                &mut stream,
                            ) {
                                warn!("error registering: {:?}", why);
                                res_tx
                                    .send((id, NetResponse::Error))
                                    .expect("failed to send net response");
                            }
                        }
                        NetRequest::UserLogin { username, password } => {
                            if let Err(why) = send(
                                ServerboundPacket::Login {
                                    id,
                                    username,
                                    password,
                                },
                                &mut stream,
                            ) {
                                warn!("error logging in: {:?}", why);
                                res_tx
                                    .send((id, NetResponse::Error))
                                    .expect("failed to send net response");
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
