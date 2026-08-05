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
    backend::cpal::CpalBackend,
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
};
use macroquad::{prelude::*, ui::root_ui};
use triple_buffer::Input;

use crate::{
    AssetStore, GlobalData,
    beatmap::{Beatmap, HitObject, Lane},
    input::KeyEvent,
    update::{RenderState, StateTransition},
};

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
#[derive(Debug)]
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
        F: Fn(&VecDeque<HitObject>),
    {
        func(&self.up);
        func(&self.down);
    }

    fn each_mut<F>(&mut self, func: F)
    where
        F: Fn(&mut VecDeque<HitObject>),
    {
        func(&mut self.up);
        func(&mut self.down);
    }
}

pub struct PlayingLogicData {
    beatmap: Beatmap,
    remaining_hit_objects: BinaryHeap<Reverse<HitObject>>,
    active_hit_objects: ActiveHitObjects,
    bpm: u32,
    lane_speed: u32,
    audio_clock: AudioClock,
    start: Instant, // Timestamp of when the song started
}

#[derive(Clone)]
pub struct PlayingRenderData {
    active_hit_objects: ActiveHitObjects,
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

pub fn init(beatmap: Beatmap, input_rx: Receiver<KeyEvent>) -> PlayingLogicData {
    debug!("Init Playing state with beatmap {:?}", beatmap);
    let remaining_hit_objects =
        BinaryHeap::from_iter(beatmap.hit_objects.iter().cloned().map(Reverse));
    let bpm = beatmap.bpm;

    // Clear the input queue to prepare
    input_rx.try_iter().for_each(|_| {});

    // start the audio
    let audio_clock =
        AudioClock::new(beatmap.audio_path.as_ref(), 0).expect("audio failed to start!");

    PlayingLogicData {
        audio_clock,
        beatmap,
        remaining_hit_objects,
        active_hit_objects: ActiveHitObjects::default(),
        bpm,
        lane_speed: 20,
        start: Instant::now(),
    }
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
    global_data: GlobalData,
    input_rx: Receiver<KeyEvent>,
    render_input: &mut Input<RenderState>,
) -> Option<StateTransition> {
    let time = data.audio_clock.time_ms();
    let render_up_to = render_up_to(data.lane_speed, time);

    // process incoming messages (hits) ---
    input_rx.try_iter().for_each(|e| {
        match e {
            KeyEvent::Down((char, instant)) => {
                debug!("Received input event: {:?}", e);
                // TODO: make the buttons configurable
                let judgement = match char {
                    7 | 8 => {
                        // top lane
                        hit_note(&mut data.active_hit_objects.up, instant, data.start)
                    }
                    43 | 47 => {
                        // bottom lane
                        hit_note(&mut data.active_hit_objects.down, instant, data.start)
                    }
                    _ => None,
                };
                debug!("Note judgement: {:?}", judgement);
            }
            _ => {}
        }
    });

    // TODO: update score

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
    data.active_hit_objects.each_mut(|objects| {
        loop {
            if let Some(last) = objects.front() {
                if let Some(judgement) = should_pop_note(last, time, data.lane_speed) {
                    debug!("popping note: {:?}", judgement);
                    objects.pop_front();
                    continue;
                }
            }
            break;
        }
    });

    // FIXME: this has poor performance as the active_hit_objects vecs are cloned each update
    render_input.write(RenderState::Playing(PlayingRenderData {
        active_hit_objects: data.active_hit_objects.clone(),
        time,
        bpm: data.bpm,
        lane_speed: data.lane_speed,
        keys_down: (true, false),
        score: 100000,
        accuracy: 98.5,
    }));
    None
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
    data.active_hit_objects.each(|objects| {
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
}
