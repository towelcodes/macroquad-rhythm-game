use std::net::TcpStream;

use crate::{
    data::GameConfig,
    util::ui::{self, AnchorPoint},
};
use crossbeam_channel::{Receiver, Sender};
use macroquad::prelude::*;
use rhythm_game_server::protocol::User;

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
        Ok(stream) => {
            info!("connected to backend! {:?}", stream);
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
