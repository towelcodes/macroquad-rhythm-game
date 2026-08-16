use crossbeam_channel::Sender;
use macroquad::prelude::*;
use std::{sync::OnceLock, time::Instant};

#[cfg(target_os = "macos")]
use core_graphics2::event::{__CGEventTapProxy, CGEvent, CGEventType};

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::Threading::{
        GetCurrentThreadId, OpenThread, SetThreadPriority, THREAD_ALL_ACCESS,
        THREAD_PRIORITY_HIGHEST,
    },
    UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, MSG,
        SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN,
        WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    },
};

#[cfg(target_os = "windows")]
use std::mem::zeroed;

pub static INPUT_SENDER: OnceLock<Sender<KeyEvent>> = OnceLock::new();

#[derive(Debug)]
pub enum KeyEvent {
    Down((u64, Instant)),
    Up((u64, Instant)),
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code == HC_ACTION as i32 {
        let kbd = unsafe { &*(l_param.0 as *const KBDLLHOOKSTRUCT) };
        let keycode = kbd.vkCode as u64;
        match w_param.0 as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                INPUT_SENDER
                    .get()
                    .unwrap()
                    .send(KeyEvent::Down((keycode, Instant::now())))
                    .unwrap();
            }
            WM_KEYUP | WM_SYSKEYUP => {
                INPUT_SENDER
                    .get()
                    .unwrap()
                    .send(KeyEvent::Up((keycode, Instant::now())))
                    .unwrap();
            }
            _ => {}
        }
    }
    // Always pass the event through so the rest of the system still sees it.
    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

// FIXME This implementation sends multiple down events when the key is held for a while
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

    #[cfg(target_os = "windows")]
    {
        info!("starting input loop (winapi)");
        unsafe {
            // Install a low-level keyboard hook. It runs on the thread that
            // installs it, so we must pump messages here to receive callbacks.
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), None, 0)
                .expect("Failed to install low-level keyboard hook");

            // Bump this thread's priority so key events are captured with
            // minimal latency.
            let thread = OpenThread(THREAD_ALL_ACCESS, false, GetCurrentThreadId()).unwrap();
            SetThreadPriority(thread, THREAD_PRIORITY_HIGHEST).unwrap();

            // The hook callback is only invoked while we pump messages.
            let mut msg: MSG = zeroed();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                let _ = DispatchMessageW(&msg);
            }

            // Clean up the hook when the message loop exits.
            let _ = UnhookWindowsHookEx(hook);
        }
    }

    #[cfg(target_os = "linux")]
    {
        info!("starting input loop (fallback)");
        unimplemented!()
    }
}
