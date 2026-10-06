use std::collections::HashMap;

use crossbeam_channel::{Receiver, Sender};
use macroquad::{
    prelude::*,
    ui::{
        Layout, Skin, hash, root_ui,
        widgets::{Button, Group},
    },
};
use triple_buffer::Input;

use crate::{
    GlobalData, Notification,
    beatmap::{Beatmap, BeatmapMeta, HitObject, HitObjectType, Lane},
    data::{
        GameConfig, KeyAction, load_beatmaps,
        scores::{self, Score},
    },
    input::{Key, KeyEvent},
    update::{RenderState, StateTransition},
    util::ui::{self, AnchorPoint},
};

enum UiEvent {
    SelectSong(usize),
    Start,
    MainMenu,
}

pub struct SongSelectLogicData {
    beatmaps: Vec<Beatmap>,
    selected: Option<usize>,
    ui_events: Receiver<UiEvent>,
    ui_events_sender: Sender<UiEvent>,
    notify_tx: Sender<Notification>,
    leaderboard_data: Option<(usize, Vec<Score>)>,
    active_beatmap_hash: Option<[u8; 32]>,
}

#[derive(Clone)]
pub struct SongSelectRenderData {
    beatmaps: Vec<Beatmap>,
    selected: Option<usize>,
    ui_events_sender: Sender<UiEvent>,
    leaderboard_data: Option<(usize, Vec<Score>)>,
    active_beatmap_hash: Option<[u8; 32]>,
}

pub fn init(config: &GameConfig, notify_tx: Sender<Notification>) -> SongSelectLogicData {
    let (ui_events_sender, ui_events) = crossbeam_channel::unbounded();

    // load songs from the directory
    let beatmaps = match load_beatmaps(&config.song_folder) {
        Ok(beatmaps) => {
            info!("loaded {} beatmaps successfully", beatmaps.len());
            beatmaps
        }
        Err(why) => {
            warn!("failed to load beatmaps. the list of beatmaps will be empty. {why:?}");
            Vec::new()
        }
    };

    SongSelectLogicData {
        beatmaps,
        selected: None,
        ui_events,
        ui_events_sender,
        notify_tx,
        leaderboard_data: None,
        active_beatmap_hash: None,
    }
}

pub fn close(data: &mut SongSelectLogicData) {}

pub fn update(
    data: &mut SongSelectLogicData,
    input_rx: Receiver<KeyEvent>,
    keybinds: &HashMap<Key, KeyAction>,
    render_input: &mut Input<RenderState>,
) -> Option<StateTransition> {
    for event in data.ui_events.try_iter() {
        match event {
            UiEvent::SelectSong(index) => {
                data.selected = Some(index);
            }
            UiEvent::Start => {
                if let Some(index) = data.selected {
                    let beatmap = data.beatmaps.remove(index);
                    return Some(StateTransition::StartBeatmap(beatmap));
                }
            }
            UiEvent::MainMenu => {
                return Some(StateTransition::MainMenu);
            }
        }
    }

    while let Ok(event) = input_rx.try_recv() {
        if let KeyEvent::Down((key, _)) = event {
            if let Some(action) = keybinds.get(&key) {
                match action {
                    KeyAction::Exit => {
                        return Some(StateTransition::MainMenu);
                    }
                    KeyAction::Confirm => {
                        if let Some(index) = data.selected {
                            let beatmap = data.beatmaps.remove(index);
                            return Some(StateTransition::StartBeatmap(beatmap));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // load scores if they aren't already loaded
    if let Some(index) = data.selected {
        if data.leaderboard_data.is_none() || data.leaderboard_data.as_ref().unwrap().0 != index {
            let beatmap = &data.beatmaps[index];
            match scores::get_scores_on(&beatmap.meta) {
                Ok(scores) => {
                    info!(
                        "got {} scores for beatmap {}",
                        scores.len(),
                        beatmap.meta.title
                    );
                    data.leaderboard_data = Some((index, scores));
                }
                Err(why) => {
                    let _ = data.notify_tx.send(Notification {
                        content:
                            "Failed to get scores for beatmap! Check the logs for more details."
                                .to_string(),
                        color: RED,
                    });
                    warn!(
                        "failed to get scores for beatmap {}: {:?}",
                        beatmap.meta.title, why
                    );
                }
            };
            data.active_beatmap_hash = Some(beatmap.hash());
        }
    }

    render_input.write(RenderState::SongSelect(SongSelectRenderData {
        beatmaps: data.beatmaps.clone(),
        selected: data.selected,
        ui_events_sender: data.ui_events_sender.clone(),
        leaderboard_data: data.leaderboard_data.clone(),
        active_beatmap_hash: data.active_beatmap_hash,
    }));
    None
}

pub async fn render(data: &SongSelectRenderData) {
    set_default_camera();

    clear_background(WHITE);

    // set the UI skin
    let label_style = root_ui().style_builder().font_size(24).build();
    let skin = Skin {
        label_style,
        ..root_ui().default_skin()
    };
    root_ui().push_skin(&skin);

    // title
    ui::label((vec2(0.5, 0.1), AnchorPoint::Centre), "Select a Song");

    // --- left: meta information box + right: scrollable song list ---
    // The `root_ui()` borrow is scoped to this block so it is released before
    // the `util::ui` helpers below (which call `root_ui()` again) are used.
    {
        let (w, h) = (screen_width(), screen_height());
        let mut ui = root_ui();

        // left: meta information box
        let selected = data.selected.and_then(|i| data.beatmaps.get(i));
        Group::new(hash!("meta"), vec2(w * 0.3, h * 0.30))
            .position(vec2(w * 0.05, h * 0.05))
            .layout(Layout::Vertical)
            .ui(&mut ui, |ui| {
                ui.label(None, "Song Information");
                ui.label(None, "");
                match selected {
                    Some(beatmap) => {
                        ui.label(None, &format!("Title: {}", beatmap.meta.title));
                        ui.label(None, &format!("Artist: {}", beatmap.meta.artist));
                        ui.label(None, &format!("Mapper: {}", beatmap.meta.mapper));
                        ui.label(
                            None,
                            &format!(
                                "Difficulty: {:.1} {}",
                                beatmap.meta.level, beatmap.meta.level_name
                            ),
                        );
                        ui.label(None, &format!("BPM: {}", beatmap.bpm));
                    }
                    None => {
                        ui.label(None, "No song selected");
                    }
                }
            });

        Group::new(hash!("leaderboard"), vec2(w * 0.3, h * 0.5))
            .position(vec2(w * 0.05, h * 0.36))
            .layout(Layout::Vertical)
            .ui(&mut ui, |ui| {
                ui.label(None, "Leaderboard");
                if let Some((_, scores)) = &data.leaderboard_data {
                    for (n, score) in scores.iter().enumerate() {
                        ui.label(
                            None,
                            &format!(
                                "#{} | {} - {:.2}% ({})",
                                n + 1,
                                score.score,
                                score.accuracy * 100.0,
                                score.by.name
                            ),
                        );

                        // show warning if the play was not completed fullyS
                        if score.early_quit {
                            let label_style = ui.style_builder()
                                .text_color(RED)
                                .font_size(12)
                                .build();
                            let skin = Skin {
                                label_style,
                                ..ui.default_skin()
                            };
                            ui.push_skin(&skin);
                            ui.label(None, "[!] This play was not completed fully.");
                            ui.pop_skin();
                        }

                        // show warning if the hash differs
                        if let Some(hash) = data.active_beatmap_hash {
                            if hash != score.beatmap_hash {
                                // make the font smaller
                                let label_style = ui.style_builder()
                                    .text_color(RED)
                                    .font_size(12)
                                    .build();
                                let skin = Skin {
                                    label_style,
                                    ..ui.default_skin()
                                };
                                ui.push_skin(&skin);
                                ui.label(None, "[!] This score was achieved on a different version of the beatmap.");
                                ui.pop_skin();
                            }
                        }
                    }
                }
            });

        // right: scrollable song list, buttons aligned to the right edge
        let list_width = w * 0.55;
        let list_height = h * 0.6;
        let list_pos = vec2(w * 0.4, h * 0.2);
        let margin = 10.0;
        let button_width = list_width * 0.7;
        let button_height = 30.0;
        let row_gap = 5.0;
        Group::new(hash!("song_list"), vec2(list_width, list_height))
            .position(list_pos)
            .layout(Layout::Vertical)
            .ui(&mut ui, |ui| {
                for (i, beatmap) in data.beatmaps.iter().enumerate() {
                    let is_selected = data.selected == Some(i);
                    // right-align: x is the group width minus the button width and margin
                    let x = list_width - button_width - margin;
                    let y = margin + i as f32 * (button_height + row_gap);
                    if Button::new(beatmap.meta.title.as_str())
                        .position(vec2(x, y))
                        .size(vec2(button_width, button_height))
                        .selected(is_selected)
                        .ui(ui)
                    {
                        if let Err(why) = data.ui_events_sender.send(UiEvent::SelectSong(i)) {
                            warn!("error sending ui event: {why:?}");
                        }
                    }
                }
            });
    }

    if ui::button((vec2(0.45, 0.85), AnchorPoint::Centre), "Main Menu") {
        if let Err(why) = data.ui_events_sender.send(UiEvent::MainMenu) {
            warn!("error sending ui event: {why:?}");
        }
    }

    // start button (only enabled once a song is selected)
    if data.selected.is_some() {
        if ui::button((vec2(0.55, 0.85), AnchorPoint::Centre), "Start") {
            if let Err(why) = data.ui_events_sender.send(UiEvent::Start) {
                warn!("error sending ui event: {why:?}");
            }
        }
    }

    root_ui().pop_skin();
}
