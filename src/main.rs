use generational_arena::{Arena, Index};
use macroquad::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::{
    thread,
    time::{Duration, Instant},
};

use crate::entity::*;
use crate::input::{KeyEvent, input_loop};
use crate::state::{GameState, StateMachine, UpdateLoop};
use crate::tween::{Tween, TweenEasing};

#[cfg(test)]
mod tests;

mod beatmap;
mod entity;
mod input;
mod state;
mod tween;

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

/*
 * TODO:
 */

/// Handles events
/// TODO sync with audio
fn update_loop(
    global_data: GlobalData,
    hud_arena: EntityArena,
    world_arena: EntityArena,
    tx: Sender<u32>,
    input_rx: Receiver<KeyEvent>,
    update_loop: &mut Box<impl UpdateLoop>,
) {
    // create FPS counter and guides
    let mut fps_counter: Option<Index> = None;
    let mut guides: Option<Index> = None;
    let mut world_guides: Option<Index> = None;
    {
        let mut guard = world_arena.write().unwrap();
        world_guides = Some(guard.insert(Box::new(WorldGuides {})));
    }

    {
        let mut guard = hud_arena.write().unwrap();
        fps_counter = Some(guard.insert(Box::new(FpsCounter::new(Arc::clone(&global_data)))));
        guides = Some(guard.insert(Box::new(GridGuides {})));
    }

    update_loop.update();

    loop {
        match input_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => match event {
                KeyEvent::Down((keycode, _instant)) => {
                    if keycode == 18 {
                        let mut guard = hud_arena.write().unwrap();
                        if let Some(i) = fps_counter.take() {
                            guard.remove(i);
                        } else {
                            fps_counter = Some(
                                guard.insert(Box::new(FpsCounter::new(Arc::clone(&global_data)))),
                            );
                        }
                    } else if keycode == 19 {
                        let mut guard = hud_arena.write().unwrap();
                        if let Some(i) = guides.take() {
                            guard.remove(i);
                        } else {
                            guides = Some(guard.insert(Box::new(GridGuides {})));
                        }
                    }
                    info!("key down: {}", keycode);
                }
                KeyEvent::Up((keycode, _instant)) => {
                    info!("key up: {}", keycode);
                }
            },
            Err(_) => {}
        };
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Game".to_owned(),
        window_width: 1280,
        window_height: 720,
        ..Default::default()
    }
}

#[derive(Default)]
pub struct Data {
    debug_lines: Mutex<Vec<String>>,
}

pub type GlobalData = Arc<Data>;
pub type EntityArena = Arc<RwLock<Arena<Box<dyn Entity>>>>;

#[macroquad::main(window_conf)]
async fn main() {
    info!("starting up...");

    let target_fps = 60;
    let frame_duration = Duration::from_secs_f32(1.0 / target_fps as f32);
    let mut last_frame = Instant::now();

    // global data
    let global_data: GlobalData = Arc::new(Data::default());

    // arena for entities
    let hud_arena: EntityArena = Arc::new(RwLock::new(Arena::new()));
    let world_arena: EntityArena = Arc::new(RwLock::new(Arena::new()));

    // input loop
    let (input_tx, input_rx) = mpsc::channel();
    thread::spawn(move || input_loop(input_tx));
    let input_rx_rc = Rc::new(RefCell::new(input_rx));

    // state machine
    let mut state_machine = StateMachine::new(
        Arc::clone(&global_data),
        Arc::clone(&world_arena),
        Arc::clone(&hud_arena),
    );

    // let (update_tx, update_rx) = mpsc::channel();
    // let global_data_clone = Arc::clone(&global_data);
    // let hud_arena_clone = Arc::clone(&hud_arena);
    // let world_arena_clone = Arc::clone(&world_arena);

    // thread::spawn(move || {
    //     update_loop(
    //         global_data_clone,
    //         hud_arena_clone,
    //         world_arena_clone,
    //         update_tx,
    //         input_rx,
    //         &state_machine,
    //     )
    // });

    // let note_texture = load_texture("textures/note.png").await.unwrap();
    // arena.insert(Box::new(Sprite {
    //     texture: note_texture.weak_clone(),
    //     x: screen_width() / 2.0,
    //     y: screen_height() / 2.0,
    // }));

    // let render_target = render_target(320, 150);

    let mut camera = Camera2D {
        zoom: vec2(1., screen_width() / screen_height()),
        ..Default::default()
    };

    info!("{:?}", camera);
    // let mut camera_tween = Tween::new(0.0, 360.0, Duration::from_secs(2), TweenEasing::EaseOut);
    let mut cam_tween_x = Tween::new(2., 1., Duration::from_millis(600), TweenEasing::EaseOut);
    let mut cam_tween_y = Tween::new(
        (screen_width() / screen_height()) + 1.,
        screen_width() / screen_height(),
        Duration::from_millis(600),
        TweenEasing::EaseOut,
    );

    loop {
        // clear_background(WHITE);

        // let centre_x = screen_width() / 2.0;
        // let centre_y = screen_height() / 2.0;

        // set_camera(&camera);

        // camera space
        // draw_circle_lines(centre_x - 60.0, centre_y + 90.0, 40.0, 4.0, BLACK);
        // draw_circle_lines(centre_x - 60.0, centre_y - 90.0, 40.0, 4.0, BLACK);
        // draw_circle_lines(centre_x + 60.0, centre_y + 90.0, 40.0, 4.0, BLACK);
        // draw_circle_lines(centre_x + 60.0, centre_y - 90.0, 40.0, 4.0, BLACK);
        // draw_circle_lines(-0.15, 0.2, 0.1, 0.01, BLACK);
        // draw_circle_lines(-0.15, -0.2, 0.1, 0.01, BLACK);
        // draw_circle_lines(0.15, 0.2, 0.1, 0.01, BLACK);
        // draw_circle_lines(0.15, -0.2, 0.1, 0.01, BLACK);

        // render world entities
        // TODO: will be handed off to the current gamestate
        // {
        //     let guard = world_arena.read().unwrap();
        //     for (_idx, value) in guard.iter() {
        //         value.draw();
        //     }
        // }

        // set_default_camera();
        // render HUD entities
        // TODO: will be handed off to the current gamestate instead
        // {
        //     let guard = hud_arena.read().unwrap();
        //     for (_idx, value) in guard.iter() {
        //         value.draw();
        //     }
        // }

        // draw active state
        state_machine.draw().await;

        // camera debug text
        {
            let mut debug_lines = global_data.debug_lines.lock().unwrap();
            *debug_lines = vec![
                format!("camera zoom x {:?}", cam_tween_x.get()),
                format!("camera zoom y {:?}", cam_tween_y.get()),
            ];
        }

        // move camera
        // camera.zoom = vec2(cam_tween_x.get(), cam_tween_y.get());
        // camera.rotation = camera_tween.get();

        // always in main render loop
        // limit fps
        let elapsed = last_frame.elapsed();
        if elapsed < frame_duration {
            let sleep_duration = frame_duration - elapsed;
            thread::sleep(sleep_duration);
        }
        last_frame = Instant::now();

        next_frame().await
    }
}
