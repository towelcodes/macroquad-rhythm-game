use arc_swap::ArcSwap;
use macroquad::miniquad::conf::Platform;
use macroquad::ui::widgets::Texture;
use macroquad::ui::{Skin, StyleBuilder, root_ui};
use macroquad::{Error, prelude::*};
use std::cell::LazyCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::{
    thread,
    time::{Duration, Instant},
};
use triple_buffer::triple_buffer;

use crate::input::input_loop;
use crate::state::*;
use crate::update::{RenderState, start_update_thread};

#[cfg(test)]
mod tests;

mod beatmap;
mod entity;
mod input;
mod state;
mod tween;
mod update;
mod util;

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
            swap_interval: Some(1), // this maybe does something idk
            ..Default::default()
        },
        ..Default::default()
    }
}

#[derive(Default)]
pub struct Data {}

/// Bundle of loaded assets
pub struct Assets {
    ui_button_bg: Image,
    note: Texture2D,
}

pub type AssetStore = LazyLock<ArcSwap<Assets>>;
static ASSETS: AssetStore = LazyLock::new(|| {
    ArcSwap::from_pointee(load_assets(Path::new("textures")).expect("Failed to load assets"))
});

#[derive(Debug, Clone, Default)]
pub struct DebugData {
    show: bool,
    update_delta: u128,
    update_target: u128,
}

pub type GlobalData = Arc<Data>;

/// Loads assets from the specified directory
pub fn load_assets(path: &Path) -> Result<Assets, Error> {
    info!("Loading assets from {path:?}");
    let chip = fs::read(path.join("chip.png")).unwrap_or_default();
    Ok(Assets {
        ui_button_bg: Image::from_file_with_format(&chip, Some(ImageFormat::Png))?,
        note: Texture2D::from_file_with_format(&chip, Some(ImageFormat::Png)),
    })
}

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
        let debug_data = debug_output.read();
        if debug_data.show {
            let fps = get_fps();
            let delta = get_frame_time();

            root_ui().label(None, &format!("Render FPS {fps}"));
            root_ui().label(None, &format!("Render Delta {delta}"));
            root_ui().label(
                None,
                &format!(
                    "Update Delta {}/{}",
                    debug_data.update_delta, debug_data.update_target
                ),
            );
        }

        match render_output.read() {
            RenderState::MainMenu(data) => main_menu::render(data, &ASSETS).await,
            RenderState::SongSelect(data) => song_select::render(data).await,
            RenderState::Playing(data) => playing::render(data, &ASSETS).await,
            RenderState::None => {}
        };

        // limit fps
        // target_duration
        //     .checked_sub(last_frame.elapsed())
        //     .map(|remaining| thread::sleep(remaining))
        //     .unwrap_or_default();
        // last_frame = Instant::now();

        next_frame().await
    }
}
