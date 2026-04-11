use std::collections::VecDeque;

use crossbeam_channel::Receiver;
use macroquad::prelude::*;
use triple_buffer::Input;

use crate::{
    GlobalData,
    beatmap::{Beatmap, HitObject},
    input::KeyEvent,
    update::StateTransition,
};

pub struct PlayingLogicData {
    beatmap: Beatmap,
    active_hit_objects: VecDeque<HitObject>,
    time: u32,
    bpm: u32,
    lane_speed: u32,
}

pub struct PlayingRenderData<'a> {
    beatmap: Beatmap,
    active_hit_objects: &'a [HitObject],
    time: u32,
    bpm: u32,
    lane_speed: u32,
}

pub fn init(beatmap: Beatmap) -> PlayingLogicData {
    PlayingLogicData {
        beatmap,
        active_hit_objects: VecDeque::new(),
        time: 0,
        bpm: beatmap.bpm,
        lane_speed: 20,
    }
}

pub fn close(data: PlayingLogicData) {}

pub fn update(
    data: &mut PlayingLogicData,
    global_data: GlobalData,
    input_rx: Receiver<KeyEvent>,
    render_input: &mut Input<PlayingRenderData>,
) -> Option<StateTransition> {
    let render_up_to = data.time + data.lane_speed * 50;

    // process incoming messages (hits) ---
    // remove them from the heap
    // render score

    // render hit objects ---
    // - peek the next hit element and check if it is in range
    loop {
        if let Some(next) = data.beatmap.hit_objects.peek() {
            if next.0.time <= render_up_to {
                data.active_hit_objects
                    .push_back(data.beatmap.hit_objects.pop().unwrap().0);
                continue;
            }
        }
        break;
    }
    // - remove old hit objects
    loop {
        if let Some(last) = data.active_hit_objects.front() {
            if last.time + data.lane_speed * 50 < data.time {
                data.active_hit_objects.pop_front();
                continue;
            }
        }
        break;
    }

    render_input.write(PlayingRenderData {
        beatmap: data.beatmap.clone(),
        active_hit_objects: &data.active_hit_objects.as_slices().0, // TODO check this?
        time: data.time,
        bpm: data.bpm,
        lane_speed: data.lane_speed,
    });
    None
}

pub async fn render<'a>(data: &PlayingRenderData<'a>) {
    // - render hit objects in the arena
    for entity in data.active_hit_objects {
        // Calculate the position of the hit object based on its time
        let time_offset = entity.time as i32 - data.time as i32;
        let y_position = (time_offset as f32 / (data.lane_speed * 50) as f32) * screen_height();

        // Render the hit object at the calculated position
        let x_position = entity.lane as u8 as f32 * (screen_width() / 4.0) + (screen_width() / 8.0);

        draw_circle(x_position, y_position, 20.0, WHITE);
    }
}
