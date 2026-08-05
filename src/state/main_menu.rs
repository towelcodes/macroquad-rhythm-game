use std::{collections::BinaryHeap, time::Duration};

use crossbeam_channel::{Receiver, Sender};
use macroquad::{
    prelude::*,
    ui::{Skin, Style, root_ui},
};
use triple_buffer::Input;

use crate::{
    AssetStore, Assets, GlobalData,
    beatmap::{Beatmap, BeatmapMeta, HitObject, HitObjectType, Lane},
    input::KeyEvent,
    tween::{Tween, TweenEasing, TweenState},
    update::{RenderState, StateTransition},
    util::ui::{self, AnchorPoint},
};

enum UiEvent {
    StartBeatmap,
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
    for event in data.ui_events.try_iter() {
        match event {
            UiEvent::StartBeatmap => {
                // really this should look through the beatmap database for the one with the corresponding ID
                // or something

                // temporary
                let mut hit_objects: Vec<HitObject> = vec![];
                for i in 0..100 {
                    hit_objects.push(HitObject {
                        time: i * 500,
                        lane: if i % 2 == 0 { Lane::Up } else { Lane::Down },
                        kind: HitObjectType::Chip,
                    });
                }
                let beatmap = Beatmap {
                    meta: BeatmapMeta {
                        title: "Exit This Earth's Atmosphere".to_string(),
                        artist: "Camellia".to_string(),
                        mapper: "teatowel".to_string(),
                        level_name: "evil".to_string(),
                        level: 9.5,
                    },
                    bpm: 200,
                    beats_per_bar: 4,
                    audio_path: "music.wav".to_owned(),
                    hit_objects,
                };
                info!("starting beatmap");
                return Some(StateTransition::StartBeatmap(beatmap));
            }
        }
    }

    let (x, y) = (data.x.get(), data.y.get());

    render_input.write(RenderState::MainMenu(MainMenuRenderData {
        offset: (x, y),
        ui_events_sender: data.ui_events_sender.clone(),
    }));

    None
}

pub async fn render(data: &MainMenuRenderData, assets: &AssetStore) {
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

    if ui::button((vec2(0.5, 0.5), AnchorPoint::Centre), "Start") {
        trace!("click");
        if let Err(why) = data.ui_events_sender.send(UiEvent::StartBeatmap) {
            warn!("error sending ui event: {why:?}");
        }
    }

    root_ui().pop_skin();
}
