use generational_arena::{Arena, Index};
use macroquad::miniquad::conf::Platform;
use macroquad::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::{
    thread,
    time::{Duration, Instant},
};
use triple_buffer::triple_buffer;

use crate::entity::*;
use crate::input::{KeyEvent, input_loop};
use crate::state::*;
use crate::tween::{Tween, TweenEasing};
use crate::update::{RenderState, start_update_thread};

#[cfg(test)]
mod tests;

mod beatmap;
mod entity;
mod input;
mod state;
mod tween;
mod update;

/*
main thread has rendering logic
- input polling thread
- timing thread
*/

/*
 * use: priority queue for hit objects (Min-Heap)
 * automated beat detection
 * linear regression for difficulty calculation
 *  use rayon to calculate difficulty using a threadpool for efficiency
 * use Finite State Machine for switching between gameplay states
 * ensure Deterministic Timing / inputs are processed in order relative to audio clock
 * use linear interpolation to estimate input latency and interpolate where the hit should register
 * replay system (virtualise inputs based on file format)
 * editor: use grid-based partitioning system for editor's timeline
 * handle hitobject lifecycles e.g. deconstruct note when offscreen
 * Generational Arena or Slot Map for entities
 *
 * docs: explain how to avoid race conditions
*/

fn window_conf() -> Conf {
    Conf {
        window_title: "Game".to_owned(),
        window_width: 1280,
        window_height: 720,
        platform: Platform {
            swap_interval: Some(0), // vsync
            ..Default::default()
        },
        ..Default::default()
    }
}

#[derive(Default)]
pub struct Data {}

#[derive(Debug, Clone, Default)]
pub struct DebugData {
    show: bool,
    update_delta: u128,
    update_target: u128,
}

pub type GlobalData = Arc<Data>;

#[macroquad::main(window_conf)]
async fn main() {
    info!("starting up...");

    // global data
    let global_data: GlobalData = Arc::new(Data::default());

    // render data buffer
    let (render_input, mut render_output) = triple_buffer(&RenderState::None);

    // debug data buffer
    let (mut debug_input, mut debug_output) = triple_buffer(&DebugData::default());

    // input loop
    let (input_tx, input_rx) = crossbeam_channel::unbounded();
    thread::Builder::new()
        .name("input".to_string())
        .spawn(move || input_loop(input_tx))
        .expect("Failed to spawn input thread");

    // update thread
    thread::Builder::new()
        .name("update".to_string())
        .spawn(move || {
            start_update_thread(
                Arc::clone(&global_data),
                input_rx,
                render_input,
                &mut debug_input,
            )
        })
        .expect("Failed to spawn update thread");

    let target_fps = 120.0;
    let target_duration = Duration::from_secs_f32(1.0 / target_fps);
    let mut last_frame = Instant::now();

    loop {
        match render_output.read() {
            RenderState::MainMenu(data) => main_menu::render(data).await,
            RenderState::SongSelect(data) => song_select::render(data).await,
            RenderState::Playing(data) => playing::render(data).await,
            RenderState::None => {}
        };

        set_default_camera();
        let debug_data = debug_output.read();
        if debug_data.show {
            let fps = get_fps();
            let delta = get_frame_time();
            draw_text(&format!("Render FPS {fps}"), 10.0, 20.0, 20.0, BLACK);
            draw_text(&format!("Render Delta {delta}"), 10.0, 40.0, 20.0, BLACK);
            draw_text(
                &format!(
                    "Update Delta {}/{}",
                    debug_data.update_delta, debug_data.update_target
                ),
                10.0,
                60.0,
                20.0,
                BLACK,
            );
        }

        // limit fps
        target_duration
            .checked_sub(last_frame.elapsed())
            .map(|remaining| thread::sleep(remaining))
            .unwrap_or_default();
        last_frame = Instant::now();

        next_frame().await
    }
}
