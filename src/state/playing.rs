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

#[derive(Clone, Default)]
struct ActiveHitObjects {
    left_up: VecDeque<HitObject>,
    right_up: VecDeque<HitObject>,
    left_down: VecDeque<HitObject>,
    right_down: VecDeque<HitObject>,
}
impl ActiveHitObjects {
    /// Utility function to run a funciton on each lane
    fn each<F>(&self, func: F)
    where
        F: Fn(&VecDeque<HitObject>),
    {
        func(&self.left_up);
        func(&self.right_up);
        func(&self.left_down);
        func(&self.right_down);
    }

    fn each_mut<F>(&mut self, func: F)
    where
        F: Fn(&mut VecDeque<HitObject>),
    {
        func(&mut self.left_up);
        func(&mut self.right_up);
        func(&mut self.left_down);
        func(&mut self.right_down);
    }
}

pub struct PlayingLogicData {
    beatmap: Beatmap,
    remaining_hit_objects: BinaryHeap<Reverse<HitObject>>,
    active_hit_objects: ActiveHitObjects,
    render_hit_objects: ActiveHitObjects,
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
        render_hit_objects: ActiveHitObjects::default(),
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
                    Lane::LeftUp => {
                        data.active_hit_objects
                            .left_up
                            .push_back(data.remaining_hit_objects.pop().unwrap().0);
                    }
                    Lane::RightUp => {
                        data.active_hit_objects
                            .right_up
                            .push_back(data.remaining_hit_objects.pop().unwrap().0);
                    }
                    Lane::LeftDown => {
                        data.active_hit_objects
                            .left_down
                            .push_back(data.remaining_hit_objects.pop().unwrap().0);
                    }
                    Lane::RightDown => {
                        data.active_hit_objects
                            .right_down
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
                if last.time + data.lane_speed * 50 < data.time {
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

    // swap the working state to the buffer to be pushed to render
    mem::swap(&mut data.active_hit_objects, &mut data.render_hit_objects);

    'update_render: {
        let render_state = render_input.input_buffer_mut();
        let RenderState::Playing(render_data) = render_state else {
            // render state has not been initialised; copy everything this time only
            trace!("initialising render state to playing");
            break 'update_render;
        };
        mem::swap(
            &mut render_data.active_hit_objects,
            &mut data.render_hit_objects,
        );
        render_data.time = data.time;
        render_data.bpm = data.bpm;
        render_data.lane_speed = data.lane_speed;
        render_data.keys_down = (true, false);
        render_data.score = 100000;
        render_data.accuracy = 98.5;
    }

    // render_input.write(RenderState::Playing(PlayingRenderData {
    //     active_hit_objects: data.active_hit_objects,
    //     time: data.time,
    //     bpm: data.bpm,
    //     lane_speed: data.lane_speed,
    //     keys_down: (true, false),
    //     score: 100000,
    //     accuracy: 98.5,
    // }));
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

    // render score TODO

    // render circles
    draw_circle_lines(-0.8, 0.2, 0.06, 0.005, BLACK);
    draw_circle_lines(-0.8, -0.2, 0.06, 0.005, BLACK);

    if data.keys_down.0 {
        draw_circle(-0.8, 0.2, 0.06, Color::new(0., 0., 0., 0.5));
    }
    if data.keys_down.1 {
        draw_circle(-0.8, -0.2, 0.06, Color::new(0., 0., 0., 0.5));
    }

    data.active_hit_objects.each(|objects| {
        for object in objects {
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
    });
}
