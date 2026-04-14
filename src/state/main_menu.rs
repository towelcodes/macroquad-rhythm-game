use std::time::Duration;

use crossbeam_channel::Receiver;
use macroquad::{prelude::*, ui::root_ui};
use triple_buffer::Input;

use crate::{
    GlobalData,
    input::KeyEvent,
    tween::{Tween, TweenEasing, TweenState},
    update::{RenderState, StateTransition},
};

pub struct MainMenuLogicData {
    x: Tween<f32>,
    y: Tween<f32>,
}

#[derive(Clone)]
pub struct MainMenuRenderData {
    offset: (f32, f32),
}

/// Run when initialiing the state (blocks update thread)
pub fn init() -> MainMenuLogicData {
    MainMenuLogicData {
        x: Tween::new(0., 0.3, Duration::from_secs(1), TweenEasing::EaseOut),
        y: Tween::new(0., 0.02, Duration::from_secs(1), TweenEasing::EaseOut),
    }
}

/// Run when closing the state (blocks update thread)
pub fn close(data: &MainMenuLogicData) {}

pub fn update(
    data: &mut MainMenuLogicData,
    global_data: GlobalData,
    input_rx: Receiver<KeyEvent>,
    render_input: &mut Input<RenderState>,
) -> Option<StateTransition> {
    // if the tween is complete, change direction
    if *data.x.state() == TweenState::Finished {
        if data.x.target() == 0.3 {
            data.x = Tween::new(0.3, -0.3, Duration::from_secs(1), TweenEasing::EaseOut);
            data.y = Tween::new(0.02, -0.02, Duration::from_secs(1), TweenEasing::EaseOut);
        } else {
            data.x = Tween::new(-0.3, 0.3, Duration::from_secs(1), TweenEasing::EaseOut);
            data.y = Tween::new(-0.02, 0.02, Duration::from_secs(1), TweenEasing::EaseOut);
        }
    }

    let (x, y) = (data.x.get(), data.y.get());

    render_input.write(RenderState::MainMenu(MainMenuRenderData { offset: (x, y) }));
    None
}

pub async fn render(data: &MainMenuRenderData) {
    let (x, y) = data.offset;

    let camera = Camera2D {
        zoom: vec2(1., screen_width() / screen_height()),
        offset: vec2(x, y),
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
}
