use std::{
    cmp::Reverse,
    collections::{BinaryHeap, VecDeque},
    error::Error,
    path::Path,
    time::Instant,
};

use crossbeam_channel::Receiver;
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend,
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
};
use macroquad::{prelude::*, ui::root_ui};
use triple_buffer::Input;

use crate::{
    AssetStore, GlobalData,
    beatmap::{Beatmap, HitObject, Lane},
    data::{GameConfig, KeyAction},
    input::{Key, KeyEvent},
    state::results::ResultsData,
    update::{RenderState, StateTransition},
};

// How long in ms judgements should show on the screen
const JUDGEMENT_DISPLAY_TIME: u32 = 600;

// This represents the relative width of a note circle
const NOTE_WIDTH: f32 = 0.06;

// The input timings for each judgement window
const PERFECT: u32 = 60;
const GREAT: u32 = 90;
const OK: u32 = 120;
const BAD: u32 = 150;
const MISS: u32 = 200;

/// An enum representing a note judgement.
/// The number is the positive or negative error in ms
#[derive(Debug, Clone)]
pub enum Judgement {
    Perfect(i32),
    Great(i32),
    Ok(i32),
    Bad(i32),
    Miss(i32),
}

#[derive(Clone, Default)]
struct ActiveHitObjects {
    up: VecDeque<HitObject>,
    down: VecDeque<HitObject>,
}
impl ActiveHitObjects {
    /// Utility function to run a funciton on each lane
    fn each<F>(&self, func: F)
    where
        F: Fn(&VecDeque<HitObject>, Lane),
    {
        func(&self.up, Lane::Up);
        func(&self.down, Lane::Down);
    }

    fn each_mut<F>(&mut self, mut func: F)
    where
        F: FnMut(&mut VecDeque<HitObject>, Lane),
    {
        func(&mut self.up, Lane::Up);
        func(&mut self.down, Lane::Down);
    }
}

#[derive(Clone, Default)]
struct ActiveJudgements {
    up: VecDeque<(Judgement, u32)>,
    down: VecDeque<(Judgement, u32)>,
}
impl ActiveJudgements {
    /// Utility function to run a funciton on each lane
    fn each<F>(&self, func: F)
    where
        F: Fn(&VecDeque<(Judgement, u32)>, Lane),
    {
        func(&self.up, Lane::Up);
        func(&self.down, Lane::Down);
    }

    fn each_mut<F>(&mut self, func: F)
    where
        F: Fn(&mut VecDeque<(Judgement, u32)>, Lane),
    {
        func(&mut self.up, Lane::Up);
        func(&mut self.down, Lane::Down);
    }
}

pub struct PlayingLogicData {
    beatmap: Beatmap,
    remaining_hit_objects: BinaryHeap<Reverse<HitObject>>,
    active_hit_objects: ActiveHitObjects,
    judgements: Vec<(Judgement, u32)>, // judgements are stored with the timestamp they were taken
    active_judgements: ActiveJudgements, // active judgements are the judgements that will be shown on the screen
    bpm: u32,
    lane_speed: u32,
    audio_clock: AudioClock,
    start: Instant, // Timestamp of when the song started
    score: u32,
}

#[derive(Clone)]
pub struct PlayingRenderData {
    active_hit_objects: ActiveHitObjects,
    active_judgements: ActiveJudgements,
    time: u32,
    bpm: u32,
    lane_speed: u32,
    keys_down: (bool, bool),
    score: u32,
    accuracy: f32,
}

pub struct AudioClock {
    manager: AudioManager,
    sound: StaticSoundHandle,
    offset_ms: i32,
}

impl AudioClock {
    pub fn new(audio_path: &Path, offset_ms: i32) -> Result<Self, Box<dyn Error>> {
        let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())?;
        let sound_data = StaticSoundData::from_file(audio_path)?;
        let sound = manager.play(sound_data)?;
        Ok(Self {
            manager,
            sound,
            offset_ms,
        })
    }

    pub fn time_ms(&self) -> u32 {
        (self.sound.position() * 1000.0 + self.offset_ms as f64).max(0.0) as u32
    }
}

pub fn init(
    config: &GameConfig,
    beatmap: Beatmap,
    input_rx: Receiver<KeyEvent>,
) -> Result<PlayingLogicData, Box<dyn Error>> {
    debug!("Init Playing state with beatmap {:?}", beatmap);
    let remaining_hit_objects =
        BinaryHeap::from_iter(beatmap.hit_objects.iter().cloned().map(Reverse));
    let bpm = beatmap.bpm;

    // Clear the input queue to prepare
    input_rx.try_iter().for_each(|_| {});

    // start the audio, resolved relative to the songs folder
    let audio_path = Path::new(&config.song_folder).join(&beatmap.audio_path);
    let audio_clock = AudioClock::new(&audio_path, 0)?;

    Ok(PlayingLogicData {
        audio_clock,
        beatmap,
        remaining_hit_objects,
        active_hit_objects: ActiveHitObjects::default(),
        active_judgements: ActiveJudgements::default(),
        bpm,
        lane_speed: 20,
        start: Instant::now(),
        judgements: vec![],
        score: 0,
    })
}

pub fn close(data: &PlayingLogicData) {
    debug!("Closing Playing state");
}

/// Returns true if the given note should be removed from the active notes queue.
/// If the note should be popped, a Judgement is returned.
fn should_pop_note(note: &HitObject, time: u32, lane_speed: u32) -> Option<Judgement> {
    // FIXME: Should calculate it properly here instead of using another function
    if calculate_note_position(note, time, lane_speed).0 < (-1.0 - NOTE_WIDTH) {
        Some(Judgement::Miss(-(MISS as i32)))
    } else {
        None
    }
}

/// Checks if a note has been hit, and if it has, pops it and returns the judgement
fn hit_note(
    lane_queue: &mut VecDeque<HitObject>,
    input_time: Instant,
    start: Instant,
) -> Option<Judgement> {
    // check for a nearby hit object
    let Some(next) = lane_queue.front() else {
        debug!("nothing at the front of the queue, skipping");
        return None;
    };

    let difference = next.time as i32 - (input_time - start).as_millis() as i32; // +ve means early, -ve means late

    debug!(
        "difference={:?} note_time={:?} relative_time={:?} input_time={:?} start={:?}",
        difference,
        next.time,
        (input_time - start).as_millis(),
        input_time,
        start
    );

    // pop the note (if required) and return the judgement
    if difference.abs() as u32 <= PERFECT {
        let _ = lane_queue.pop_front();
        Some(Judgement::Perfect(difference))
    } else if difference.abs() as u32 <= GREAT {
        let _ = lane_queue.pop_front();
        Some(Judgement::Great(difference))
    } else if difference.abs() as u32 <= OK {
        let _ = lane_queue.pop_front();
        Some(Judgement::Ok(difference))
    } else if difference.abs() as u32 <= BAD {
        let _ = lane_queue.pop_front();
        Some(Judgement::Bad(difference))
    } else {
        None
    }
}

fn render_up_to(lane_speed: u32, time: u32) -> u32 {
    time + lane_speed * 50
}

pub fn update(
    data: &mut PlayingLogicData,
    input_rx: Receiver<KeyEvent>,
    render_input: &mut Input<RenderState>,
    config: &GameConfig,
) -> Option<StateTransition> {
    let time = data.audio_clock.time_ms();
    let render_up_to = render_up_to(data.lane_speed, time);
    let (mut top_lane_down, mut bottom_lane_down) = (false, false);

    // flag to return early
    let mut quit = false;

    // process incoming messages (hits) ---
    input_rx.try_iter().for_each(|e| {
        match e {
            KeyEvent::Down((key, instant)) => {
                debug!("Received input event: {:?}", e);

                let action = config.keybinds.get(&key);
                if action.is_none() {
                    return;
                }
                let action = action.unwrap();

                // escape returns to the results screen early
                if *action == KeyAction::Exit {
                    quit = true;
                    return;
                }

                // top lane
                let judgement = match action {
                    KeyAction::LaneUp | KeyAction::LaneUpAlt => {
                        top_lane_down = true;
                        hit_note(&mut data.active_hit_objects.up, instant, data.start)
                    }
                    _ => None,
                };
                if let Some(judgement) = judgement {
                    debug!("Note judgement: {:?}", judgement);
                    add_score(&data.beatmap, &judgement, &mut data.score);
                    data.judgements.push((judgement.clone(), time));
                    data.active_judgements.up.push_back((judgement, time));
                    return;
                }

                // bottom lane
                let judgement = match action {
                    KeyAction::LaneDown | KeyAction::LaneDownAlt => {
                        bottom_lane_down = true;
                        hit_note(&mut data.active_hit_objects.down, instant, data.start)
                    }
                    _ => None,
                };
                if let Some(judgement) = judgement {
                    debug!("Note judgement: {:?}", judgement);
                    add_score(&data.beatmap, &judgement, &mut data.score);
                    data.judgements.push((judgement.clone(), time));
                    data.active_judgements.down.push_back((judgement, time));
                }
            }
            KeyEvent::Up((key, _)) => {
                debug!("Received input event: {:?}", e);
                if let Some(action) = config.keybinds.get(&key) {
                    match action {
                        KeyAction::LaneUp | KeyAction::LaneUpAlt => top_lane_down = false,
                        KeyAction::LaneDown | KeyAction::LaneDownAlt => bottom_lane_down = false,
                        _ => {}
                    }
                }
            }
        }
    });

    // return to results early if escape was pressed
    if quit {
        return Some(StateTransition::Results(ResultsData {
            score: data.score,
            accuracy: calculate_accuracy(&data.judgements),
            judgements: data.judgements.clone(),
            beatmap: data.beatmap.clone(),
        }));
    }

    // check for expired judgements
    data.active_judgements.each_mut(|queue, _| {
        while let Some((_, created)) = queue.front() {
            if time.saturating_sub(*created) > JUDGEMENT_DISPLAY_TIME {
                queue.pop_front();
            } else {
                break;
            }
        }
    });

    // hit objects ---
    // - peek the next hit element and check if it is in range
    loop {
        if let Some(next) = data.remaining_hit_objects.peek() {
            if next.0.time <= render_up_to {
                match next.0.lane {
                    Lane::Up => {
                        data.active_hit_objects
                            .up
                            .push_back(data.remaining_hit_objects.pop().unwrap().0);
                    }
                    Lane::Down => {
                        data.active_hit_objects
                            .down
                            .push_back(data.remaining_hit_objects.pop().unwrap().0);
                    }
                }
                continue;
            }
        }
        break;
    }
    // - remove old hit objects
    data.active_hit_objects.each_mut(|objects, lane| {
        loop {
            if let Some(last) = objects.front() {
                if let Some(judgement) = should_pop_note(last, time, data.lane_speed) {
                    debug!("popping note: {:?}", judgement);
                    objects.pop_front();
                    data.judgements.push((judgement.clone(), time));
                    match lane {
                        Lane::Up => {
                            data.active_judgements.up.push_back((judgement, time));
                        }
                        Lane::Down => {
                            data.active_judgements.down.push_back((judgement, time));
                        }
                    }
                    continue;
                }
            }
            break;
        }
    });

    // FIXME: this has poor performance as the active_hit_objects vecs are cloned each update
    render_input.write(RenderState::Playing(PlayingRenderData {
        active_hit_objects: data.active_hit_objects.clone(),
        active_judgements: data.active_judgements.clone(),
        time,
        bpm: data.bpm,
        lane_speed: data.lane_speed,
        keys_down: (top_lane_down, bottom_lane_down),
        score: data.score,
        accuracy: calculate_accuracy(&data.judgements),
    }));

    // transition to results once every note has been played
    if data.remaining_hit_objects.is_empty()
        && data.active_hit_objects.up.is_empty()
        && data.active_hit_objects.down.is_empty()
    {
        return Some(StateTransition::Results(ResultsData {
            score: data.score,
            accuracy: calculate_accuracy(&data.judgements),
            judgements: data.judgements.clone(),
            beatmap: data.beatmap.clone(),
        }));
    }

    None
}

// note: these functions need to be pub(crate) for unit tests
/// Calculates the amount to add to a score
/// given the beatmap and judgement
pub(crate) fn add_score(beatmap: &Beatmap, judgement: &Judgement, score: &mut u32) {
    // the max score you can get on any song is 1,000,000
    // (perfect on all notes)
    // FIXME it's possible for 1000000 to not be perfectly divisible, so it could be impossible to get 1000000 (WRITE about that + copy osu)
    let max_score_per_note = 1000000 / beatmap.hit_objects.len() as u32;
    *score += match judgement {
        Judgement::Perfect(_) => max_score_per_note,
        Judgement::Great(_) => (max_score_per_note / 4) * 3,
        Judgement::Ok(_) => (max_score_per_note / 4) * 2,
        Judgement::Bad(_) => max_score_per_note / 4,
        Judgement::Miss(_) => 0,
    };
}

/// Calculates the accuracy of the player based on the judgements taken
/// Returns 1.0 (100%) when no judgements have been taken yet
pub(crate) fn calculate_accuracy(judgements: &[(Judgement, u32)]) -> f32 {
    if judgements.is_empty() {
        return 1.0;
    }

    let total: f32 = judgements
        .iter()
        .map(|(judgement, _)| match judgement {
            Judgement::Perfect(_) => 1.0,
            Judgement::Great(_) => 0.75,
            Judgement::Ok(_) => 0.5,
            Judgement::Bad(_) => 0.25,
            Judgement::Miss(_) => 0.0,
        })
        .sum();

    total / judgements.len() as f32
}

/// Calculates the position a note should be on screen
/// given the current time and lane speed.
fn calculate_note_position(note: &HitObject, time: u32, lane_speed: u32) -> (f32, f32) {
    // this is the time in future up to which notes should be shown
    // the end of the screen will show notes at this amount of time in the future (ms)
    let screen_end = render_up_to(lane_speed, time) - time;

    // Calculate the position of the hit object based on its time
    let time_offset = note.time as f32 - time as f32;
    let x_offset = (time_offset / screen_end as f32) * 1.8;

    let x_position = -0.8 + x_offset;
    let y_position = match note.lane {
        Lane::Up => -0.2,
        Lane::Down => 0.2,
    };

    (x_position, y_position)
}

/// Draws a judgement on the screen, given the time it was created and the lane
fn draw_judgement(judgement: &Judgement, lane: Lane, created: u32, time: u32) {
    let age = time.saturating_sub(created);
    let alpha = 1.0 - (age as f32 / JUDGEMENT_DISPLAY_TIME as f32);

    let (text, colour) = match judgement {
        Judgement::Perfect(_) => ("Perfect", Color::from_hex(0x74c7ec).with_alpha(alpha)),
        Judgement::Great(_) => ("Great", Color::from_hex(0xa6e3a1).with_alpha(alpha)),
        Judgement::Ok(_) => ("Ok", Color::from_hex(0xf9e2af).with_alpha(alpha)),
        Judgement::Bad(_) => ("Bad", Color::from_hex(0xeba0ac).with_alpha(alpha)),
        Judgement::Miss(_) => ("Miss", Color::from_hex(0xf38ba8).with_alpha(alpha)),
    };

    let font_size = 40.0;
    let (width, height) = (screen_width(), screen_height());
    let dims = measure_text(text, None, font_size as u16, 1.0);
    let y = match lane {
        Lane::Up => (height / 2.0) - (height * 0.2),
        Lane::Down => (height / 2.0) + (height * 0.2),
    };
    root_ui().label(
        None,
        &format!(
            "J: text={:?} x={:?} y={:?} font_size={:?}",
            text,
            (0.2 * width) - dims.width / 2.0,
            y + dims.height / 2.0,
            font_size
        ),
    );
    draw_text(
        text,
        (0.2 * width) - dims.width / 2.0,
        y + dims.height / 2.0,
        font_size,
        colour,
    );
}

pub async fn render(data: &PlayingRenderData, assets: &AssetStore) {
    let render_up_to = render_up_to(data.lane_speed, data.time);
    let screen_end = render_up_to - data.time;

    clear_background(WHITE);

    // debug text
    root_ui().label(None, &format!("time: {}", data.time));
    root_ui().label(
        None,
        &format!(
            "render_up_to={}ms screen_end={}ms lane_speed={}",
            render_up_to, screen_end, data.lane_speed
        ),
    );

    // set the camera so we can use relative positions
    let camera = Camera2D {
        zoom: vec2(1., screen_width() / screen_height()),
        ..Default::default()
    };

    set_camera(&camera);

    // render score TODO

    // render circles
    draw_circle_lines(-0.8, 0.2, NOTE_WIDTH, 0.005, BLACK);
    draw_circle_lines(-0.8, -0.2, NOTE_WIDTH, 0.005, BLACK);

    if data.keys_down.0 {
        draw_circle(-0.8, 0.2, NOTE_WIDTH, Color::new(0., 0., 0., 0.5));
    }
    if data.keys_down.1 {
        draw_circle(-0.8, -0.2, NOTE_WIDTH, Color::new(0., 0., 0., 0.5));
    }

    // render notes
    data.active_hit_objects.each(|objects, _| {
        for object in objects {
            let (x_position, y_position) =
                calculate_note_position(object, data.time, data.lane_speed);
            draw_circle(x_position, y_position, 0.05, BLACK);

            // debug text
            root_ui().label(
                None,
                &format!("HO: t={} x={} y={}", data.time, x_position, y_position),
            );
        }
    });

    // return to screen space
    set_default_camera();

    // render judgements
    data.active_judgements.each(|queue, lane| {
        queue.iter().for_each(|(judgement, created)| {
            draw_judgement(&judgement, lane, *created, data.time);
        });
    });

    // render score and accuracy
    let width = screen_width();
    draw_text(
        &format!("{:07}", data.score),
        width / 2.0,
        40.0,
        40.0,
        BLACK,
    );
    draw_text(
        &format!("{:.2}", data.accuracy * 100.0),
        width / 2.0,
        60.0,
        40.0,
        BLACK,
    );
}
