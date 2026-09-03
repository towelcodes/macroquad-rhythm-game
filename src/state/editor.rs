use std::{
    collections::VecDeque,
    error::Error,
    sync::{Arc, Mutex},
};

use kira::{AudioManager, AudioManagerSettings, sound::static_sound::StaticSoundHandle};
use macroquad::{
    color::WHITE,
    prelude::*,
    ui::{
        Layout, Skin, hash, root_ui,
        widgets::{Editbox, Group, Window},
    },
};
use triple_buffer::Input;

use crate::{
    beatmap::{Beatmap, HitObject},
    state::playing::{NOTE_WIDTH, calculate_note_position, render_up_to, should_pop_note},
    update::{RenderState, StateTransition},
    util::ui::format_time,
};

enum SnapPoints {
    Half,
    Quarter,
}

pub struct EditorState {
    /// current playback pos (ms)
    time: u32,
    playing: bool,
    seek: f32,
    /// the lane speed is like the zoom
    lane_speed: u32,

    show_metadata: bool,
    active_beatmap: Beatmap,

    // some settings need to be saved in strings for the UI,
    // they will be converted/validated when saving
    bpm_text: String,
    level_text: String,

    // the hit objects
    // this will be edited by a function which changes the time
    past_hit_objects: VecDeque<HitObject>,
    current_hit_objects: VecDeque<HitObject>,
    future_hit_objects: VecDeque<HitObject>,

    manager: AudioManager,
    active_audio: Option<StaticSoundHandle>,
    snap_points: SnapPoints,
}

pub struct EditorLogicData {
    state: Arc<Mutex<EditorState>>,
}

#[derive(Clone)]
pub struct EditorRenderData {
    state: Arc<Mutex<EditorState>>,
}

pub fn init() -> Result<EditorLogicData, Box<dyn Error>> {
    Ok(EditorLogicData {
        state: Arc::new(Mutex::new(EditorState {
            time: 0,
            playing: false,
            seek: 0.0,
            lane_speed: 20,

            show_metadata: false,
            active_beatmap: Beatmap::default(),

            bpm_text: "120".to_string(),
            level_text: "1.0".to_string(),

            past_hit_objects: VecDeque::new(),
            current_hit_objects: VecDeque::new(),
            future_hit_objects: VecDeque::new(),

            snap_points: SnapPoints::Half,
            manager: AudioManager::new(AudioManagerSettings::default())?,
            active_audio: None,
        })),
    })
}

pub fn open_beatmap(state: &mut EditorState) {}

/// Collects all hit objects across the three queues
/// and updates the beatmap data in the editor state
fn collect_hit_objects(state: &mut EditorState) {
    let mut objects: Vec<HitObject> = state
        .past_hit_objects
        .iter()
        .chain(state.current_hit_objects.iter())
        .chain(state.future_hit_objects.iter())
        .cloned()
        .collect();
    objects.sort();
    state.active_beatmap.hit_objects = objects;
}

fn save_to_file(state: &mut EditorState, path: &std::path::Path) -> Result<(), Box<dyn Error>> {
    // before saving collect all the hit objects
    collect_hit_objects(state);

    // save the text as numbers
    state.active_beatmap.bpm = state.bpm_text.parse().unwrap_or_else(|err| {
        warn!("could not parse bpm, falling back to 120: {:?}", err);
        120
    });
    state.active_beatmap.meta.level = state.bpm_text.parse().unwrap_or_else(|err| {
        warn!("could not parse level, falling back to 0.0: {:?}", err);
        0.0
    });

    let contents =
        ron::ser::to_string_pretty(&state.active_beatmap, ron::ser::PrettyConfig::default())?;
    std::fs::write(path, contents)?;
    Ok(())
}

fn load_from_file(state: &mut EditorState, path: &std::path::Path) -> Result<(), Box<dyn Error>> {
    let contents = std::fs::read_to_string(path)?;
    state.active_beatmap = ron::de::from_str(&contents)?;

    // load the numbers as text
    state.bpm_text = format!("{}", state.active_beatmap.bpm);
    state.level_text = format!("{}", state.active_beatmap.meta.level);

    // load all objects into the future queue; they will be pulled into the
    // current queue as time advances.
    state.time = 0;
    state.future_hit_objects = state.active_beatmap.hit_objects.clone().into();
    state.current_hit_objects.clear();
    state.past_hit_objects.clear();
    Ok(())
}

pub fn update(
    data: &mut EditorLogicData,
    render_input: &mut Input<RenderState>,
) -> Option<StateTransition> {
    // TODO: when implemented, deterministically advance `state.time` here while
    // `playing` is true, e.g. based on the elapsed time since the last tick.

    render_input.write(RenderState::Editor(EditorRenderData {
        state: data.state.clone(),
    }));
    None
}

pub fn render(data: &EditorRenderData) {
    let mut state = data.state.lock().unwrap();

    set_default_camera();
    clear_background(WHITE);

    // set the UI skin
    let label_style = root_ui().style_builder().font_size(24).build();
    let skin = Skin {
        label_style,
        ..root_ui().default_skin()
    };
    root_ui().push_skin(&skin);

    let (w, h) = (screen_width(), screen_height());
    let bar_height = 40.0;

    // --- top menu bar
    // widgets in macroquad's UI default to a vertical layout when given no
    // position, so we place each one explicitly to lay them out horizontally
    {
        let mut ui = root_ui();
        let mut x = 8.0;
        let y = 5.0;
        let widget_h = 30.0;

        // use native file picker
        if ui.button(vec2(x, y), "Save") {
            // if let Some(path) = rfd::FileDialog::new()
            //     .add_filter("Beatmap", &["ron"])
            //     .set_file_name("beatmap.ron")
            //     .save_file()
            // {
            //     if let Err(why) = save_to_file(&mut state, &path) {
            //         error!("failed to save beatmap: {why:?}");
            //     }
            // }
        }
        x += 70.0;
        if ui.button(vec2(x, y), "Load") {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Beatmap", &["ron"])
                .pick_file()
            {
                if let Err(why) = load_from_file(&mut state, &path) {
                    error!("failed to load beatmap: {why:?}");
                }
            }
        }
        x += 70.0;
        if ui.button(vec2(x, y), "Export") {
            // TODO: export the current beatmap
        }
        x += 90.0;
        if ui.button(vec2(x, y), "Edit Metadata") {
            state.show_metadata = !state.show_metadata;
        }
        x += 150.0;

        // BPM text input
        ui.label(vec2(x, y + 3.0), "BPM");
        x += 45.0;
        let mut bpm = state.bpm_text.clone();
        Editbox::new(hash!("bpm-input"), vec2(60., widget_h))
            .position(vec2(x, y))
            .multiline(false)
            .ui(&mut *ui, &mut bpm);
        if bpm != state.bpm_text {
            state.bpm_text = bpm;
        }
        x += 70.0;

        // current time display
        ui.label(vec2(x, y + 3.0), &format_time(state.time));
        x += 110.0;

        // play/pause button
        let label = if state.playing { "Pause" } else { "Play" };
        if ui.button(vec2(x, y), label) {
            state.playing = !state.playing;
        }
    }

    // --- bottom seek bar
    // The slider widget doesn't accept an explicit position (it lays itself out
    // using the active window's cursor), so wrap it in a Group pinned to the
    // bottom of the screen.
    {
        let mut ui = root_ui();
        Group::new(hash!("seek-bar"), vec2(w, bar_height))
            .position(vec2(0., h - bar_height))
            .layout(Layout::Horizontal)
            .ui(&mut ui, |ui| {
                let mut seek = state.seek;
                ui.slider(hash!("seek"), "Seek", 0.0..1.0, &mut seek);
                if seek != state.seek {
                    state.seek = seek;
                }
            });
    }

    // metadata editing window
    if state.show_metadata {
        let mut ui = root_ui();

        // local mutable copies so the editboxes can edit them, written back to
        // the shared state once the window is drawn.
        let mut title = state.active_beatmap.meta.title.clone();
        let mut artist = state.active_beatmap.meta.artist.clone();
        let mut mapper = state.active_beatmap.meta.mapper.clone();
        let mut level = state.level_text.clone();
        let mut bpm = state.bpm_text.clone();
        let mut audio_path = state.active_beatmap.audio_path.clone();

        let opened = Window::new(
            hash!("metadata"),
            vec2(w * 0.3, h * 0.2),
            vec2(w * 0.4, h * 0.55),
        )
        .label("Beatmap Metadata")
        .close_button(true)
        .ui(&mut ui, |ui| {
            ui.label(None, "Title");
            Editbox::new(hash!("title"), vec2(300., 30.))
                .multiline(false)
                .ui(ui, &mut title);
            ui.label(None, "Artist");
            Editbox::new(hash!("artist"), vec2(300., 30.))
                .multiline(false)
                .ui(ui, &mut artist);
            ui.label(None, "Mapper");
            Editbox::new(hash!("mapper"), vec2(300., 30.))
                .multiline(false)
                .ui(ui, &mut mapper);
            ui.label(None, "Level");
            Editbox::new(hash!("level"), vec2(300., 30.))
                .multiline(false)
                .ui(ui, &mut level);
            ui.label(None, "BPM");
            Editbox::new(hash!("bpm"), vec2(300., 30.))
                .multiline(false)
                .ui(ui, &mut bpm);
            ui.label(None, "Audio File");
            Editbox::new(hash!("audio"), vec2(300., 30.))
                .multiline(false)
                .ui(ui, &mut audio_path);
        });

        // write any edited fields back to the shared state directly
        if title != state.active_beatmap.meta.title {
            state.active_beatmap.meta.title = title;
        }
        if artist != state.active_beatmap.meta.artist {
            state.active_beatmap.meta.artist = artist;
        }
        if mapper != state.active_beatmap.meta.mapper {
            state.active_beatmap.meta.mapper = mapper;
        }
        if level != state.level_text {
            // this is a number; validate the input
            if level.parse::<f32>().is_ok() {
                state.level_text = level;
            }
        }
        if bpm != state.bpm_text {
            // this is also a number; validate the input
            if bpm.parse::<u32>().is_ok() {
                state.bpm_text = bpm;
            }
        }
        if audio_path != state.active_beatmap.audio_path {
            state.active_beatmap.audio_path = audio_path;
        }

        // keep the window open unless the user closed it via the close button
        if !opened {
            state.show_metadata = false;
        }
    }

    root_ui().pop_skin();

    // FIXME this is not the proper way to progress time it should be synced to the audio clock
    if state.playing {
        state.time += (get_frame_time() * 1000.0) as u32;
    }

    // -----
    // check time, swap notes in or out
    // TODO this will need to work both ways as time may increase or decrease
    // TODO Probably use binary heap for performance
    let render_up_to = render_up_to(state.lane_speed, state.time);

    // add future notes
    loop {
        if state.future_hit_objects.front().is_none() {
            break;
        }
        if state.future_hit_objects.front().unwrap().time > render_up_to {
            break;
        }
        let object = state.future_hit_objects.pop_front().unwrap();
        state.current_hit_objects.push_back(object);
    }

    // remove old notes
    loop {
        if let Some(last) = state.current_hit_objects.front() {
            if should_pop_note(last, state.time, state.lane_speed).is_some() {
                let object = state.current_hit_objects.pop_front().unwrap();
                state.past_hit_objects.push_front(object);
                continue;
            }
        }
        break;
    }

    // ------
    // start drawing the notes

    // set the camera so we can use relative positions
    let camera = Camera2D {
        zoom: vec2(1., screen_width() / screen_height()),
        ..Default::default()
    };
    set_camera(&camera);

    draw_circle_lines(-0.8, 0.2, NOTE_WIDTH, 0.005, BLACK);
    draw_circle_lines(-0.8, -0.2, NOTE_WIDTH, 0.005, BLACK);

    // read notes from the queue and display them
    for object in &state.current_hit_objects {
        let (x_position, y_position) =
            calculate_note_position(&object, state.time, state.lane_speed);
        draw_circle(x_position, y_position, 0.05, BLACK);

        // debug text
        root_ui().label(
            None,
            &format!("HO: t={} x={} y={}", state.time, x_position, y_position),
        );
    }
}

pub fn close(data: &EditorLogicData) {}
