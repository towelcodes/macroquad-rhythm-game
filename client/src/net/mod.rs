use std::net::TcpStream;

use crate::{
    data::GameConfig,
    util::ui::{self, AnchorPoint},
};
use crossbeam_channel::{Receiver, Sender};
use macroquad::prelude::*;
use rhythm_game_server::protocol::{ClientboundPacket, User, recv, send};

struct OnlineUser {
    active_token: [u8; 32],
    user: User,
}

#[derive(Debug, Clone)]
pub enum NetEvent {}

#[derive(Debug, Clone)]
pub enum NetCommand {}

const BACKEND_ADDRESS: &str = "127.0.0.1:7300";
pub fn start_net_thread(net_event_tx: Sender<NetEvent>, net_cmd_rx: Receiver<NetCommand>) {
    // create connection
    match TcpStream::connect(BACKEND_ADDRESS) {
        Ok(mut stream) => {
            info!("connected to backend! {:?}", stream);
            // send ping
            if let Err(why) = send(ClientboundPacket::Ping, &mut stream) {
                warn!("error sending ping: {:?}", why);
            }

            loop {
                match recv::<ClientboundPacket>(&mut stream) {
                    Ok(packet) => {
                        info!("received packet: {:?}", packet);
                        if packet == ClientboundPacket::Pong {
                            info!("received pong from server");
                        } else if packet == ClientboundPacket::Ping {
                            info!("received ping from server, sending pong");
                            if let Err(why) = send(ClientboundPacket::Pong, &mut stream) {
                                warn!("error sending pong: {:?}", why);
                            }
                        }
                    }
                    Err(why) => {
                        warn!("error receiving packet: {:?}", why);
                        break;
                    }
                }
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
