use std::{
    cmp::Reverse,
    collections::{BinaryHeap, VecDeque},
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

pub struct PlayingLogicData {
    beatmap: Beatmap,
    remaining_hit_objects: BinaryHeap<Reverse<HitObject>>,
    active_hit_objects: VecDeque<HitObject>,
    time: u32,
    last_update: Instant,
    bpm: u32,
    lane_speed: u32,
}

#[derive(Clone)]
pub struct PlayingRenderData {
    active_hit_objects: Vec<HitObject>,
    time: u32,
    bpm: u32,
    lane_speed: u32,
}

pub fn init(beatmap: Beatmap) -> PlayingLogicData {
    debug!("Init Playing state with beatmap {:?}", beatmap);
    let remaining_hit_objects =
        BinaryHeap::from_iter(beatmap.hit_objects.iter().cloned().map(Reverse));
    let bpm = beatmap.bpm;
    PlayingLogicData {
        beatmap,
        remaining_hit_objects,
        active_hit_objects: VecDeque::new(),
        time: 0,
        last_update: Instant::now(),
        bpm,
        lane_speed: 20,
    }
}

pub fn close(data: &PlayingLogicData) {
    debug!("Closing Playing state");
}

pub fn update(
    data: &mut PlayingLogicData,
    global_data: GlobalData,
    input_rx: Receiver<KeyEvent>,
    render_input: &mut Input<RenderState>,
) -> Option<StateTransition> {
    // see keys
    input_rx.try_iter().for_each(|e| {
        debug!("Received input event: {:?}", e);
    });

    let render_up_to = data.time + data.lane_speed * 50;

    // process incoming messages (hits) ---
    // remove them from the heap
    // render score

    // render hit objects ---
    // - peek the next hit element and check if it is in range
    loop {
        if let Some(next) = data.remaining_hit_objects.peek() {
            if next.0.time <= render_up_to {
                data.active_hit_objects
                    .push_back(data.remaining_hit_objects.pop().unwrap().0);
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

    // increment time
    let now = Instant::now();
    if let Some(elapsed) = now.checked_duration_since(data.last_update) {
        data.time += elapsed.as_millis() as u32;
        data.last_update = now;
    }

    render_input.write(RenderState::Playing(PlayingRenderData {
        active_hit_objects: data.active_hit_objects.clone().into(), // TODO check this?
        time: data.time,
        bpm: data.bpm,
        lane_speed: data.lane_speed,
    }));
    None
}

pub async fn render(data: &PlayingRenderData, assets: &AssetStore) {
    let render_up_to = data.time + data.lane_speed * 50;
    let screen_end = render_up_to - data.time;

    clear_background(WHITE);
    root_ui().label(None, &format!("time: {}", data.time));
    root_ui().label(
        None,
        &format!(
            "render_up_to={}ms screen_end={}ms lane_speed={}",
            render_up_to, screen_end, data.lane_speed
        ),
    );

    let camera = Camera2D {
        zoom: vec2(1., screen_width() / screen_height()),
        ..Default::default()
    };

    set_camera(&camera);

    // render circles
    draw_circle_lines(-0.8, 0.2, 0.06, 0.005, BLACK);
    draw_circle_lines(-0.8, -0.2, 0.06, 0.005, BLACK);

    for object in &data.active_hit_objects {
        // Calculate the position of the hit object based on its time
        let time_offset = object.time as f32 - data.time as f32;
        let x_offset = (time_offset / screen_end as f32) * 1.8;

        let x_position = -0.8 + x_offset;
        let y_position = match object.lane {
            Lane::LeftUp => 0.2,
            Lane::RightUp => 0.2,
            Lane::LeftDown => -0.2,
            Lane::RightDown => -0.2,
        };

        draw_circle(x_position, y_position, 0.05, BLACK);
        root_ui().label(
            None,
            &format!(
                "HO: t={} offset={} x={} y={}",
                object.time, x_offset, x_position, y_position
            ),
        )
    }
}
