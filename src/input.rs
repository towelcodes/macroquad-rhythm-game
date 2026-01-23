use macroquad::prelude::*;
use std::{
    sync::{OnceLock, mpsc::Sender},
    time::Instant,
};

#[cfg(target_os = "macos")]
use core_graphics2::event::{__CGEventTapProxy, CGEvent, CGEventType};

pub static INPUT_SENDER: OnceLock<Sender<KeyEvent>> = OnceLock::new();

pub enum KeyEvent {
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
pub fn input_loop(tx: Sender<KeyEvent>) {
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
