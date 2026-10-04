use std::{collections::BinaryHeap, time::Duration};

use crossbeam_channel::{Receiver, Sender};
use macroquad::{
    prelude::*,
    ui::{
        Skin, Style, hash, root_ui,
        widgets::{Editbox, Group, Slider, Window},
    },
};
use triple_buffer::Input;

use crate::{
    AssetStore, Assets, GlobalData,
    beatmap::{Beatmap, BeatmapMeta, HitObject, HitObjectType, Lane},
    data::GameConfig,
    input::KeyEvent,
    update::{RenderState, StateTransition},
    util::{
        tween::{Tween, TweenEasing, TweenState},
        ui::{self, AnchorPoint},
    },
};

enum UiEvent {
    Play,
    Edit,
    Quit,
    OpenSettings,
    CloseSettings,
    SetLaneSpeed(u32),
    SetSongsFolder(String),
}

pub struct MainMenuLogicData {
    x: Tween<f32>,
    y: Tween<f32>,
    ui_events: Receiver<UiEvent>,
    ui_events_sender: Sender<UiEvent>,
    show_settings: bool,
}

#[derive(Clone)]
pub struct MainMenuRenderData {
    offset: (f32, f32),
    ui_events_sender: Sender<UiEvent>,
    show_settings: bool,
    lane_speed: u32,
    songs_folder: String,
}

/// Run when initialiing the state (blocks update thread)
pub fn init() -> MainMenuLogicData {
    let (ui_events_sender, ui_events) = crossbeam_channel::unbounded();
    MainMenuLogicData {
        x: Tween::new(0., 0.3, Duration::from_secs(1), TweenEasing::EaseOut),
        y: Tween::new(0., 0.02, Duration::from_secs(1), TweenEasing::EaseOut),
        show_settings: false,
        ui_events,
        ui_events_sender,
    }
}

/// Run when closing the state (blocks update thread)
pub fn close(data: &MainMenuLogicData) {}

pub fn update(
    data: &mut MainMenuLogicData,
    config: &mut GameConfig,
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
    for event in data.ui_events.try_iter() {
        match event {
            UiEvent::Play => {
                info!("starting beatmap");
                return Some(StateTransition::SongSelect);
            }
            UiEvent::Quit => {
                return Some(StateTransition::Quit);
            }
            UiEvent::Edit => {
                info!("starting editor");
                return Some(StateTransition::Editor);
            }
            UiEvent::SetLaneSpeed(speed) => {
                config.lane_speed = speed;
            }
            UiEvent::SetSongsFolder(folder) => {
                config.song_folder = folder;
            }
            UiEvent::CloseSettings => {
                data.show_settings = false;
            }
            UiEvent::OpenSettings => {
                data.show_settings = true;
            }
        }
    }

    let (x, y) = (data.x.get(), data.y.get());

    render_input.write(RenderState::MainMenu(MainMenuRenderData {
        offset: (x, y),
        ui_events_sender: data.ui_events_sender.clone(),
        show_settings: data.show_settings,
        lane_speed: config.lane_speed,
        songs_folder: config.song_folder.clone(),
    }));

    None
}

fn draw_settings(data: &MainMenuRenderData) {
    let (w, h) = (screen_width(), screen_height());
    let mut ui = root_ui();
    let mut lane_speed = data.lane_speed as f32;
    let mut volume = 100.0; // this isn't saved yet
    let mut songs_folder = data.songs_folder.clone();

    let opened = Window::new(
        hash!("settings"),
        vec2(w * 0.3, h * 0.2),
        vec2(w * 0.4, h * 0.55),
    )
    .label("Settings")
    .close_button(true)
    .ui(&mut ui, |ui| {
        Group::new(hash!("lane-speed-adjust"), vec2(330., 30.)).ui(ui, |ui| {
            ui.slider(
                hash!("lane-speed"),
                "Lane Speed",
                1.0..40.0,
                &mut lane_speed,
            );
        });

        Group::new(hash!("volume-adjust"), vec2(330., 30.)).ui(ui, |ui| {
            ui.slider(hash!("volume"), "Volume", 0.0..100.0, &mut volume);
        });

        ui.label((vec2(10., 80.)), "Songs Folder");
        Editbox::new(hash!("song-folder"), vec2(60., 30.))
            .position(vec2(0., 0.))
            .multiline(false)
            .ui(&mut *ui, &mut songs_folder);
    });

    // validate input
    if lane_speed != data.lane_speed as f32 {
        if let Err(why) = data
            .ui_events_sender
            .send(UiEvent::SetLaneSpeed(lane_speed.round() as u32))
        {
            warn!("error sending ui event: {why:?}");
        }
    }

    if songs_folder != data.songs_folder {
        if let Err(why) = data
            .ui_events_sender
            .send(UiEvent::SetSongsFolder(songs_folder.clone()))
        {
            warn!("error sending ui event: {why:?}");
        }
    }

    if !opened {
        if let Err(why) = data.ui_events_sender.send(UiEvent::CloseSettings) {
            warn!("error sending ui event: {why:?}");
        }
    }
}

pub async fn render(data: &MainMenuRenderData, _assets: &AssetStore) {
    let (ox, oy) = data.offset;

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

    // set the UI skin
    let label_style = root_ui().style_builder().font_size(24).build();
    let skin = Skin {
        label_style,
        ..root_ui().default_skin()
    };
    root_ui().push_skin(&skin);

    ui::label((vec2(0.5, 0.4), AnchorPoint::Centre), "Rhythm Game");

    // settings window
    if data.show_settings {
        draw_settings(&data);
    }

    if ui::button((vec2(0.5, 0.5), AnchorPoint::Centre), "Play") {
        if let Err(why) = data.ui_events_sender.send(UiEvent::Play) {
            warn!("error sending ui event: {why:?}");
        }
    }

    if ui::button((vec2(0.5, 0.54), AnchorPoint::Centre), "Edit") {
        if let Err(why) = data.ui_events_sender.send(UiEvent::Edit) {
            warn!("error sending ui event: {why:?}");
        }
    }

    if ui::button((vec2(0.5, 0.58), AnchorPoint::Centre), "Settings") {
        if let Err(why) = data.ui_events_sender.send(UiEvent::OpenSettings) {
            warn!("error sending ui event: {why:?}");
        }
    }

    if ui::button((vec2(0.5, 0.62), AnchorPoint::Centre), "Quit") {
        if let Err(why) = data.ui_events_sender.send(UiEvent::Quit) {
            warn!("error sending ui event: {why:?}");
        }
    }

    root_ui().pop_skin();
}
