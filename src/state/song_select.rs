use crossbeam_channel::Receiver;
use macroquad::prelude::*;
use triple_buffer::Input;

use crate::{
    GlobalData,
    input::KeyEvent,
    update::{RenderState, StateTransition},
};

/// This data will be passed to the update function each update tick.
pub struct SongSelectLogicData {}

/// This data will be published by the update loop, and passed to the render function.
#[derive(Clone)]
pub struct SongSelectRenderData {}

/// This function will run when the state is initialised. It provides the initial LogicData.
pub fn init() -> SongSelectLogicData {
    SongSelectLogicData {}
}

/// This function will run when the state is transitioning away.
pub fn close(data: &mut SongSelectLogicData) {}

/// This function will be called each update tick.
pub fn update(
    data: &mut SongSelectLogicData,
    global_data: GlobalData,
    input_rx: Receiver<KeyEvent>,
    render_input: &mut Input<RenderState>,
) -> Option<StateTransition> {
    None
}

/// This function will be called on the render thread each frame.
/// It receives the RenderData published by the update loop, and draws to the screen.
pub async fn render(data: &SongSelectRenderData) {
    let camera = Camera2D {
        zoom: vec2(1., screen_width() / screen_height()),
        ..Default::default()
    };
    clear_background(WHITE);
}
