use crossbeam_channel::Receiver;
use macroquad::prelude::*;
use triple_buffer::Input;

use crate::{
    GlobalData,
    input::KeyEvent,
    update::{RenderState, StateTransition},
};

pub struct SongSelectLogicData {}

#[derive(Clone)]
pub struct SongSelectRenderData {}

pub fn init() -> SongSelectLogicData {
    SongSelectLogicData {}
}

pub fn close(data: &mut SongSelectLogicData) {}

pub fn update(
    data: &mut SongSelectLogicData,
    global_data: GlobalData,
    input_rx: Receiver<KeyEvent>,
    render_input: &mut Input<RenderState>,
) -> Option<StateTransition> {
    None
}

pub async fn render(data: &SongSelectRenderData) {
    let camera = Camera2D {
        zoom: vec2(1., screen_width() / screen_height()),
        ..Default::default()
    };
    clear_background(WHITE);
}
