use crate::util::ui::{self, AnchorPoint};
use macroquad::prelude::*;

struct User {
    active_token: [u8; 32],
    username: String,
}

struct NetState {
    active_user: Option<User>,
}

impl NetState {
    /// Draws an overlay with the connection information
    /// Must be called from the main thread
    pub fn draw_net_overlay(&self) {
        ui::label((vec2(0., 1.), AnchorPoint::BottomLeft), "LOCAL MODE");
    }
}
