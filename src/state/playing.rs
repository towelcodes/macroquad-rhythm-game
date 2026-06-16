use std::{
    cmp::Reverse,
    collections::{BinaryHeap, VecDeque},
    mem,
    time::Instant,
};

use crossbeam_channel::Receiver;
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
    time: u32,
    last_update: Instant,
    bpm: u32,
    lane_speed: u32,
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

pub fn init(beatmap: Beatmap) -> PlayingLogicData {
    debug!("Init Playing state with beatmap {:?}", beatmap);
    let remaining_hit_objects =
        BinaryHeap::from_iter(beatmap.hit_objects.iter().cloned().map(Reverse));
    let bpm = beatmap.bpm;
    PlayingLogicData {
        beatmap,
        remaining_hit_objects,
        active_hit_objects: ActiveHitObjects::default(),
        time: 0,
        last_update: Instant::now(),
        bpm,
        lane_speed: 20,
    }
}

pub fn close(data: &PlayingLogicData) {
    debug!("Closing Playing state");
}

/// Returns true if the given note should be removed from the active notes queue.
fn should_pop_note(note: &HitObject, time: u32, lane_speed: u32) -> bool {
    // FIXME: Should calculate it properly here instead of using another function
    calculate_note_position(note, time, lane_speed).0 < (-1.0 - NOTE_WIDTH)
}

pub fn update(
    data: &mut PlayingLogicData,
    global_data: GlobalData,
    input_rx: Receiver<KeyEvent>,
    render_input: &mut Input<RenderState>,
) -> Option<StateTransition> {
    let render_up_to = data.time + data.lane_speed * 50;

    // process incoming messages (hits) ---
    input_rx.try_iter().for_each(|e| {
        debug!("Received input event: {:?}", e);
        // match e {
        //     KeyEvent::Down((char, instant)) => {
        //         // TODO: make the buttons configurable
        //         match char {
        //             7 | 8 => {
        //                 // top lane
        //                 // check for a nearby hit object
        //                 let threshold = 100u32; // ms
        //                 let object = data.active_hit_objects.iter().find(|obj| {
        //                     (obj.lane == Lane::LeftUp || obj.lane == Lane::RightUp)
        //                         && obj.time.abs_diff(data.time) < threshold
        //                 });
        //             }
        //             44 | 47 => { // bottom lane
        //             }
        //             _ => {}
        //         }
        //     }
        //     _ => {}
        // }
    });
    // remove them from the heap

    // render score

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
                if should_pop_note(last, data.time, data.lane_speed) {
                    objects.pop_front();
                    continue;
                }
            }
            break;
        }
    });

    // increment time
    let now = Instant::now();
    if let Some(elapsed) = now.checked_duration_since(data.last_update) {
        data.time += elapsed.as_millis() as u32;
        data.last_update = now;
    }

    // FIXME: this has poor performance as the active_hit_objects vecs are cloned each update
    render_input.write(RenderState::Playing(PlayingRenderData {
        active_hit_objects: data.active_hit_objects.clone(),
        time: data.time,
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
    let screen_end = lane_speed * 50;

    // Calculate the position of the hit object based on its time
    let time_offset = note.time as f32 - time as f32;
    let x_offset = (time_offset / screen_end as f32) * 1.8;

    let x_position = -0.8 + x_offset;
    let y_position = match note.lane {
        Lane::Up => 0.2,
        Lane::Down => -0.2,
    };

    (x_position, y_position)
}

pub async fn render(data: &PlayingRenderData, assets: &AssetStore) {
    let render_up_to = data.time + data.lane_speed * 50;
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
