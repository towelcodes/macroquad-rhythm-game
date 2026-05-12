use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use macroquad::{
    prelude::*,
    ui::{Skin, Style, root_ui},
};
use triple_buffer::Input;

use crate::{
    AssetStore, Assets, GlobalData,
    input::KeyEvent,
    tween::{Tween, TweenEasing, TweenState},
    update::{RenderState, StateTransition},
    util::ui::{self, AnchorPoint},
};

enum UiEvent {
    StartBeatmap(u32),
}

pub struct MainMenuLogicData {
    x: Tween<f32>,
    y: Tween<f32>,
    ui_events: Receiver<UiEvent>,
    ui_events_sender: Sender<UiEvent>,
}

#[derive(Clone)]
pub struct MainMenuRenderData {
    offset: (f32, f32),
    ui_events_sender: Sender<UiEvent>,
}

/// Run when initialiing the state (blocks update thread)
pub fn init() -> MainMenuLogicData {
    let (ui_events_sender, ui_events) = crossbeam_channel::unbounded();
    MainMenuLogicData {
        x: Tween::new(0., 0.3, Duration::from_secs(1), TweenEasing::EaseOut),
        y: Tween::new(0., 0.02, Duration::from_secs(1), TweenEasing::EaseOut),
        ui_events,
        ui_events_sender,
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

    // check for ui events
    data.ui_events.try_iter().for_each(|event| match event {
        UiEvent::StartBeatmap(id) => {
            info!("starting beatmap {}", id);
        }
    });

    let (x, y) = (data.x.get(), data.y.get());

    render_input.write(RenderState::MainMenu(MainMenuRenderData {
        offset: (x, y),
        ui_events_sender: data.ui_events_sender.clone(),
    }));

    None
}

pub async fn render(data: &MainMenuRenderData, assets: &AssetStore) {
    let (ox, oy) = data.offset;
    let centre_x = screen_width() / 2.;
    let centre_y = screen_height() / 2.;

    let camera = Camera2D {
        zoom: vec2(1., screen_width() / screen_height()),
        offset: vec2(ox, oy),
        ..Default::default()
    };
    clear_background(WHITE);

    // render world entities in camera space
    set_camera(&camera);
    draw_circle_lines(-0.15, 0.2, 0.1, 0.01, BLACK);
    draw_circle_lines(-0.15, -0.2, 0.1, 0.01, BLACK);
    draw_circle_lines(0.15, 0.2, 0.1, 0.01, BLACK);
    draw_circle_lines(0.15, -0.2, 0.1, 0.01, BLACK);

    // draw ui elements
    // let assets_lock = assets.load();
    // let ui_button_bg = assets_lock.ui_button_bg.clone();
    // let button_style = root_ui().style_builder().background(ui_button_bg).build();

    // let skin = Skin {
    //     button_style,
    //     ..root_ui().default_skin()
    // };
    // root_ui().push_skin(&skin);

    // set the UI skin
    let label_style = root_ui().style_builder().font_size(24).build();
    let skin = Skin {
        label_style,
        ..root_ui().default_skin()
    };
    root_ui().push_skin(&skin);

    ui::label((vec2(0.5, 0.4), AnchorPoint::Centre), "Rhythm Game");

    if ui::button((vec2(0.5, 0.5), AnchorPoint::Centre), "Start") {
        if let Err(why) = data.ui_events_sender.send(UiEvent::StartBeatmap(0)) {
            warn!("error sending ui event: {why:?}");
        }
    }

    root_ui().pop_skin();
}
