#[cfg(target_os = "macos")]
use core_graphics2::event::{__CGEventTapProxy, CGEvent, CGEventType};

use generational_arena::{Arena, Index};
use macroquad::prelude::*;
use std::ops::{Add, BitOr, Div, Mul, Shl, Sub};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, OnceLock, RwLock};
use std::{
    thread,
    time::{Duration, Instant},
};

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

static INPUT_SENDER: OnceLock<Sender<KeyEvent>> = OnceLock::new();

enum KeyEvent {
    Down((u64, Instant)),
    Up((u64, Instant)),
}

#[cfg(target_os = "macos")]
fn cg_input_callback<'a>(
    _proxy: *const __CGEventTapProxy,
    event_type: CGEventType,
    event: &'a CGEvent,
) -> Option<CGEvent> {
    match event_type {
        CGEventType::KeyDown => {
            use core_graphics2::event::CGEventField;
            let keycode = event.get_integer_value_field(CGEventField::KeyboardEventKeycode);
            INPUT_SENDER
                .get()
                .unwrap()
                .send(KeyEvent::Down((keycode as u64, Instant::now())))
                .unwrap();
        }
        CGEventType::KeyUp => {
            use core_graphics2::event::CGEventField;
            let keycode = event.get_integer_value_field(CGEventField::KeyboardEventKeycode);
            INPUT_SENDER
                .get()
                .unwrap()
                .send(KeyEvent::Up((keycode as u64, Instant::now())))
                .unwrap();
        }
        _ => {}
    }
    None
}

/// Uses OS-specific APIs to capture low latency input.
fn input_loop(tx: Sender<KeyEvent>) {
    INPUT_SENDER
        .set(tx)
        .expect("Failed to initialise input send channel");
    #[cfg(target_os = "macos")]
    {
        info!("starting input loop (core_graphics)");
        use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes};
        use core_graphics2::event::{
            CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
        };

        let event_tap = CGEventTap::new(
            CGEventTapLocation::HIDEventTap,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            vec![CGEventType::KeyDown, CGEventType::KeyUp],
            cg_input_callback,
        )
        .expect("Failed to create event tap");
        event_tap.enable(true);

        let run_loop_source = event_tap.mach_port.create_runloop_source(0).unwrap();
        let run_loop = CFRunLoop::get_current();
        run_loop.add_source(&run_loop_source, unsafe { kCFRunLoopCommonModes });
        CFRunLoop::run_current();
    }
}

#[derive(PartialEq, Debug)]
enum TweenState {
    NotStarted,
    Playing,
    Finished,
}

/// An animation playing over time
/// TODO: Sync to audio clock
/// TODO: Use generic type
struct Tween<T> {
    value: T,
    target: T,
    start: Option<Instant>,
    end: Instant,
    state: TweenState,
}
impl<T> Tween<T>
where
    T: Sub<Output = T>
        + Add<Output = T>
        + Mul<Output = T>
        + Div<Output = T>
        + From<f32>
        + Copy
        + PartialOrd,
{
    pub fn new(value: T, target: T, duration: Duration) -> Self {
        Self {
            value,
            target,
            start: None,
            end: Instant::now()
                .checked_add(duration)
                .expect("Failed to calculate end time"),
            state: TweenState::NotStarted,
        }
    }

    pub fn state(&self) -> &TweenState {
        &self.state
    }

    /// Get the current value
    pub fn get(&mut self) -> T {
        if self.state == TweenState::Finished {
            return self.target;
        }
        if self.start.is_none() {
            self.start = Some(Instant::now());
            self.state = TweenState::Playing;
        }
        let start = self.start.unwrap();
        let multiplier = (start.elapsed().as_millis() as f32)
            / (self.end.duration_since(start).as_millis() as f32);

        if multiplier > 1f32 {
            self.state = TweenState::Finished;
            return self.target;
        }
        self.target + ((self.target - self.value) * T::from(multiplier))
    }
}

/// Handles events, in sync with audio
fn update_loop(
    hud_arena: Arc<RwLock<Arena<Box<dyn Entity>>>>,
    world_arena: Arc<RwLock<Arena<Box<dyn Entity>>>>,
    tx: Sender<u32>,
    input_rx: Receiver<KeyEvent>,
) {
    // create FPS counter and guides
    let mut fps_counter: Option<Index> = None;
    let mut guides: Option<Index> = None;
    {
        let mut guard = hud_arena.write().unwrap();
        fps_counter = Some(guard.insert(Box::new(FpsCounter {})));
        guides = Some(guard.insert(Box::new(GridGuides {})));
    }

    loop {
        match input_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => match event {
                KeyEvent::Down((keycode, _instant)) => {
                    if keycode == 18 {
                        let mut guard = hud_arena.write().unwrap();
                        if let Some(i) = fps_counter.take() {
                            guard.remove(i);
                        } else {
                            fps_counter = Some(guard.insert(Box::new(FpsCounter {})));
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
        window_title: "BasicShapes".to_owned(),
        window_width: 800,
        window_height: 600,
        ..Default::default()
    }
}

trait Entity: Send + Sync {
    fn draw(&self);
}

struct Sprite {
    texture: Texture2D, /* should be a weak copy of the texture */
    x: f32,
    y: f32,
}
impl Entity for Sprite {
    fn draw(&self) {
        draw_texture(&self.texture, self.x, self.y, WHITE);
    }
}

struct FpsCounter;
impl Entity for FpsCounter {
    fn draw(&self) {
        let fps = get_fps();
        let delta = get_frame_time();
        draw_text(&format!("FPS {fps}"), 10.0, 20.0, 20.0, BLACK);
        draw_text(&format!("Delta {delta}"), 10.0, 40.0, 20.0, BLACK);
        draw_text(&format!("1 - Toggle Guides"), 10.0, 60.0, 20.0, BLACK);
        draw_text(&format!("2 - Toggle HUD"), 10.0, 80.0, 20.0, BLACK);
    }
}

struct GridGuides;
impl Entity for GridGuides {
    fn draw(&self) {
        draw_line(0.0, 0.0, screen_width(), screen_height(), 2.0, RED);
        draw_line(0.0, screen_height(), screen_width(), 0.0, 2.0, RED);
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    info!("starting up...");

    let target_fps = 60;
    let frame_duration = Duration::from_secs_f32(1.0 / target_fps as f32);
    let mut last_frame = Instant::now();

    // arena for entities
    let hud_arena: Arc<RwLock<Arena<Box<dyn Entity>>>> = Arc::new(RwLock::new(Arena::new()));
    let world_arena: Arc<RwLock<Arena<Box<dyn Entity>>>> = Arc::new(RwLock::new(Arena::new()));

    let (input_tx, input_rx) = mpsc::channel();
    thread::spawn(move || input_loop(input_tx));

    let (update_tx, update_rx) = mpsc::channel();
    let hud_arena_clone = Arc::clone(&hud_arena);
    let world_arena_clone = Arc::clone(&world_arena);
    thread::spawn(move || update_loop(hud_arena_clone, world_arena_clone, update_tx, input_rx));

    // let note_texture = load_texture("textures/note.png").await.unwrap();
    // arena.insert(Box::new(Sprite {
    //     texture: note_texture.weak_clone(),
    //     x: screen_width() / 2.0,
    //     y: screen_height() / 2.0,
    // }));

    // let render_target = render_target(320, 150);

    let mut camera =
        Camera2D::from_display_rect(Rect::new(0., 0., screen_width(), screen_height()));

    let mut camera_tween: Tween<f32> = Tween::new(0.0, 360.0, Duration::from_secs(10));

    loop {
        clear_background(WHITE);

        let centre_x = screen_width() / 2.0;
        let centre_y = screen_height() / 2.0;

        set_camera(&camera);

        draw_circle_lines(centre_x - 60.0, centre_y + 90.0, 40.0, 4.0, BLACK);
        draw_circle_lines(centre_x - 60.0, centre_y - 90.0, 40.0, 4.0, BLACK);
        draw_circle_lines(centre_x + 60.0, centre_y + 90.0, 40.0, 4.0, BLACK);
        draw_circle_lines(centre_x + 60.0, centre_y - 90.0, 40.0, 4.0, BLACK);

        // render world entities
        {
            let guard = world_arena.read().unwrap();
            for (_idx, value) in guard.iter() {
                value.draw();
            }
        }

        set_default_camera();
        // render HUD entities
        {
            let guard = hud_arena.read().unwrap();
            for (_idx, value) in guard.iter() {
                value.draw();
            }
        }

        // move camera
        camera.rotation = camera_tween.get();

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
