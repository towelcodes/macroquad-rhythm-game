use crossbeam_channel::Receiver;
use macroquad::prelude::*;
use triple_buffer::Input;

use crate::{
    GlobalData,
    entity::Entity,
    input::KeyEvent,
    update::{RenderState, StateTransition},
};

pub struct MainMenuLogicData {}

pub struct MainMenuRenderData {
    global_data: GlobalData,
}

/// Run when initialiing the state (blocks update thread)
pub fn init() -> MainMenuLogicData {
    MainMenuLogicData {}
}

/// Run when closing the state (blocks update thread)
pub fn close(data: MainMenuLogicData) {}

pub fn update(
    data: MainMenuLogicData,
    global_data: GlobalData,
    input_rx: Receiver<KeyEvent>,
    render_input: Input<RenderState>,
) -> Option<StateTransition> {
    None
}

pub async fn render(data: &MainMenuRenderData) {
    let camera = Camera2D {
        zoom: vec2(1., screen_width() / screen_height()),
        ..Default::default()
    };
    clear_background(WHITE);
    let centre_x = screen_width() / 2.0;
    let centre_y = screen_height() / 2.0;

    // render world entities in camera space
    set_camera(&camera);
    draw_circle_lines(-0.15, 0.2, 0.1, 0.01, BLACK);
    draw_circle_lines(-0.15, -0.2, 0.1, 0.01, BLACK);
    draw_circle_lines(0.15, 0.2, 0.1, 0.01, BLACK);
    draw_circle_lines(0.15, -0.2, 0.1, 0.01, BLACK);

    {
        let guard = data.world_arena.read().unwrap();
        for (_idx, value) in guard.iter() {
            value.draw();
        }
    }

    // render HUD entities in screen space
    set_default_camera();
    {
        let guard = data.hud_arena.read().unwrap();
        for (_idx, value) in guard.iter() {
            value.draw();
        }
    }
}
