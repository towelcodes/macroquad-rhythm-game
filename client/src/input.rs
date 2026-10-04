use crossbeam_channel::Sender;
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
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

/// Platform independent Key enum
/// Provides methods to convert from the keycode on the current platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Key {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    D0,
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
    D7,
    D8,
    D9,
    Comma,
    Dot,
    Minus,
    Equals,
    Semicolon,
    Quote,
    Backslash,
    Slash,
    Backtick,
    LeftBracket,
    RightBracket,
    Space,
    Enter,
    Tab,
    Backspace,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Shift,
    Control,
    Alt,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Unknown(u64),
}

impl Key {
    /// Converts a raw OS keycode into a unified [`Key`].
    #[cfg(target_os = "macos")]
    pub fn from_keycode(keycode: u64) -> Key {
        use Key::*;
        match keycode {
            0 => A,
            1 => S,
            2 => D,
            3 => F,
            4 => H,
            5 => G,
            6 => Z,
            7 => X,
            8 => C,
            9 => V,
            11 => B,
            12 => Q,
            13 => W,
            14 => E,
            15 => R,
            16 => Y,
            17 => T,
            18 => D1,
            19 => D2,
            20 => D3,
            21 => D4,
            22 => D6,
            23 => D5,
            24 => Equals,
            25 => D9,
            26 => D7,
            27 => Minus,
            28 => D8,
            29 => D0,
            30 => RightBracket,
            31 => O,
            32 => U,
            33 => LeftBracket,
            34 => I,
            35 => Backslash,
            37 => L,
            38 => J,
            39 => Quote,
            40 => K,
            41 => Semicolon,
            42 => Backslash,
            43 => Comma,
            44 => Slash,
            45 => N,
            46 => M,
            47 => Dot,
            49 => Space,
            50 => Backtick,
            53 => Escape,
            55 => Control,
            56 => Shift,
            58 => Alt,
            59 => Control,
            60 => Shift,
            61 => Alt,
            62 => Control,
            65 => Dot,
            76 => Enter,
            82 => D0,
            83 => D1,
            84 => D2,
            85 => D3,
            86 => D4,
            87 => D5,
            88 => D6,
            89 => D7,
            91 => D8,
            92 => D9,
            115 => F1,
            116 => F2,
            117 => F3,
            118 => F4,
            119 => F5,
            120 => F6,
            121 => F7,
            122 => F8,
            123 => Left,
            124 => Right,
            125 => Down,
            126 => Up,
            other => Unknown(other),
        }
    }

    /// Converts a raw OS keycode into a [`Key`].
    #[cfg(target_os = "windows")]
    pub fn from_keycode(keycode: u64) -> Key {
        use Key::*;
        match keycode {
            0x08 => Backspace,
            0x09 => Tab,
            0x0D => Enter,
            0x10 => Shift,
            0x11 => Control,
            0x12 => Alt,
            0x1B => Escape,
            0x20 => Space,
            0x25 => Left,
            0x26 => Up,
            0x27 => Right,
            0x28 => Down,
            0x30 => D0,
            0x31 => D1,
            0x32 => D2,
            0x33 => D3,
            0x34 => D4,
            0x35 => D5,
            0x36 => D6,
            0x37 => D7,
            0x38 => D8,
            0x39 => D9,
            0x41 => A,
            0x42 => B,
            0x43 => C,
            0x44 => D,
            0x45 => E,
            0x46 => F,
            0x47 => G,
            0x48 => H,
            0x49 => I,
            0x4A => J,
            0x4B => K,
            0x4C => L,
            0x4D => M,
            0x4E => N,
            0x4F => O,
            0x50 => P,
            0x51 => Q,
            0x52 => R,
            0x53 => S,
            0x54 => T,
            0x55 => U,
            0x56 => V,
            0x57 => W,
            0x58 => X,
            0x59 => Y,
            0x5A => Z,
            0x70 => F1,
            0x71 => F2,
            0x72 => F3,
            0x73 => F4,
            0x74 => F5,
            0x75 => F6,
            0x76 => F7,
            0x77 => F8,
            0x78 => F9,
            0x79 => F10,
            0x7A => F11,
            0x7B => F12,
            0xBC => Comma,
            0xBE => Dot,
            0xBD => Minus,
            0xBB => Equals,
            0xBA => Semicolon,
            0xDE => Quote,
            0xDC => Backslash,
            0xBF => Slash,
            0xC0 => Backtick,
            0xDB => LeftBracket,
            0xDD => RightBracket,
            _ => Unknown(keycode),
        }
    }
}

#[derive(Debug)]
pub enum KeyEvent {
    Down((Key, Instant)),
    Up((Key, Instant)),
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
        let key = Key::from_keycode(keycode);
        match w_param.0 as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                INPUT_SENDER
                    .get()
                    .unwrap()
                    .send(KeyEvent::Down((key, Instant::now())))
                    .unwrap();
            }
            WM_KEYUP | WM_SYSKEYUP => {
                INPUT_SENDER
                    .get()
                    .unwrap()
                    .send(KeyEvent::Up((key, Instant::now())))
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
            let key = Key::from_keycode(keycode as u64);
            INPUT_SENDER
                .get()
                .unwrap()
                .send(KeyEvent::Down((key, Instant::now())))
                .unwrap();
        }
        CGEventType::KeyUp => {
            use core_graphics2::event::CGEventField;
            let keycode = event.get_integer_value_field(CGEventField::KeyboardEventKeycode);
            let key = Key::from_keycode(keycode as u64);
            INPUT_SENDER
                .get()
                .unwrap()
                .send(KeyEvent::Up((key, Instant::now())))
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
