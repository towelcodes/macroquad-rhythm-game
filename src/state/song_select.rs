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
    data::{GameConfig, load_beatmaps},
    input::KeyEvent,
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
}

#[derive(Clone)]
pub struct SongSelectRenderData {
    beatmaps: Vec<Beatmap>,
    selected: Option<usize>,
    ui_events_sender: Sender<UiEvent>,
}

pub fn init(config: &GameConfig) -> SongSelectLogicData {
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
    }
}

pub fn close(data: &mut SongSelectLogicData) {}

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
                    let beatmap = data.beatmaps.remove(index);
                    return Some(StateTransition::StartBeatmap(beatmap));
                }
            }
            UiEvent::MainMenu => {
                return Some(StateTransition::MainMenu);
            }
        }
    }

    render_input.write(RenderState::SongSelect(SongSelectRenderData {
        beatmaps: data.beatmaps.clone(),
        selected: data.selected,
        ui_events_sender: data.ui_events_sender.clone(),
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
        Group::new(hash!("meta"), vec2(w * 0.3, h * 0.6))
            .position(vec2(w * 0.05, h * 0.2))
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
