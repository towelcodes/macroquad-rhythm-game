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
    GlobalData,
    beatmap::{Beatmap, BeatmapMeta, HitObject, HitObjectType, Lane},
    input::KeyEvent,
    update::{RenderState, StateTransition},
    util::ui::{self, AnchorPoint},
};

/// A selectable song entry.
#[derive(Clone)]
struct Song {
    title: String,
    artist: String,
    mapper: String,
    difficulty: f32,
    bpm: u32,
}

enum UiEvent {
    SelectSong(usize),
    Start,
    MainMenu,
}

pub struct SongSelectLogicData {
    songs: Vec<Song>,
    selected: Option<usize>,
    ui_events: Receiver<UiEvent>,
    ui_events_sender: Sender<UiEvent>,
}

/// This data will be published by the update loop, and passed to the render function.
#[derive(Clone)]
pub struct SongSelectRenderData {
    songs: Vec<Song>,
    selected: Option<usize>,
    ui_events_sender: Sender<UiEvent>,
}

/// This function will run when the state is initialised. It provides the initial LogicData.
pub fn init() -> SongSelectLogicData {
    let (ui_events_sender, ui_events) = crossbeam_channel::unbounded();
    let songs = vec![
        Song {
            title: "Exit This Earth's Atmosphere".to_string(),
            artist: "Camellia".to_string(),
            mapper: "teatowel".to_string(),
            difficulty: 9.5,
            bpm: 200,
        },
        Song {
            title: "Example Song".to_string(),
            artist: "Artist".to_string(),
            mapper: "mapper".to_string(),
            difficulty: 4.0,
            bpm: 120,
        },
        Song {
            title: "Another Song".to_string(),
            artist: "Someone".to_string(),
            mapper: "someone else".to_string(),
            difficulty: 6.5,
            bpm: 160,
        },
    ];
    SongSelectLogicData {
        songs,
        selected: None,
        ui_events,
        ui_events_sender,
    }
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
    for event in data.ui_events.try_iter() {
        match event {
            UiEvent::SelectSong(index) => {
                data.selected = Some(index);
            }
            UiEvent::Start => {
                if let Some(index) = data.selected {
                    let song = &data.songs[index];
                    return Some(StateTransition::StartBeatmap(build_beatmap(song)));
                }
            }
            UiEvent::MainMenu => {
                return Some(StateTransition::MainMenu);
            }
        }
    }

    render_input.write(RenderState::SongSelect(SongSelectRenderData {
        songs: data.songs.clone(),
        selected: data.selected,
        ui_events_sender: data.ui_events_sender.clone(),
    }));
    None
}

/// Builds a placeholder beatmap for the given song.
fn build_beatmap(song: &Song) -> Beatmap {
    let mut hit_objects: Vec<HitObject> = vec![];
    for i in 0..100 {
        hit_objects.push(HitObject {
            time: i * 500,
            lane: if i % 2 == 0 { Lane::Up } else { Lane::Down },
            kind: HitObjectType::Chip,
        });
    }
    Beatmap {
        meta: BeatmapMeta {
            title: song.title.clone(),
            artist: song.artist.clone(),
            mapper: song.mapper.clone(),
            level: song.difficulty,
            level_name: "evil".to_string(),
        },
        bpm: song.bpm,
        beats_per_bar: 4,
        audio_path: "music.wav".to_owned(),
        hit_objects,
    }
}

/// This function will be called on the render thread each frame.
/// It receives the RenderData published by the update loop, and draws to the screen.
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
        let selected = data.selected.and_then(|i| data.songs.get(i));
        Group::new(hash!("meta"), vec2(w * 0.3, h * 0.6))
            .position(vec2(w * 0.05, h * 0.2))
            .layout(Layout::Vertical)
            .ui(&mut ui, |ui| {
                ui.label(None, "Song Information");
                ui.label(None, "");
                match selected {
                    Some(song) => {
                        ui.label(None, &format!("Title: {}", song.title));
                        ui.label(None, &format!("Artist: {}", song.artist));
                        ui.label(None, &format!("Mapper: {}", song.mapper));
                        ui.label(None, &format!("Difficulty: {:.1}", song.difficulty));
                        ui.label(None, &format!("BPM: {}", song.bpm));
                    }
                    None => {
                        ui.label(None, "No song selected");
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
                for (i, song) in data.songs.iter().enumerate() {
                    let is_selected = data.selected == Some(i);
                    // right-align: x is the group width minus the button width and margin
                    let x = list_width - button_width - margin;
                    let y = margin + i as f32 * (button_height + row_gap);
                    if Button::new(song.title.as_str())
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
