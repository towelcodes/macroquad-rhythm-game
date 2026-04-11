use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use crossbeam_channel::{Receiver, Sender};
use generational_arena::Index;
use macroquad::prelude::{info, state_machine};
use triple_buffer::{Input, Output};

use crate::{
    GlobalData,
    entity::{FpsCounter, GridGuides, WorldGuides},
    input::KeyEvent,
    state::{
        main_menu::{MainMenuLogicData, MainMenuRenderData},
        *,
    },
};

/// Handles events
/// TODO sync with audio
pub fn start_update_thread(
    global_data: GlobalData,
    input_rx: Receiver<KeyEvent>,
    render_input: Input<RenderState>,
) {
    // create FSM
    let mut state_machine = StateMachine::new(
        GameState::MainMenu(main_menu::init()),
        global_data,
        input_rx,
        render_input,
    );

    let delta = Duration::from_secs_f32(1.0 / 500.0); // 500hz
    let mut last = Instant::now();
    loop {
        state_machine.update();

        // avoid pinning the cpu
        delta
            .checked_sub(last.elapsed())
            .map(|remaining| thread::sleep(remaining))
            .unwrap_or_default();
        last = Instant::now();

        // match input_rx.recv_timeout(Duration::from_millis(100)) {
        //     Ok(event) => match event {
        //         KeyEvent::Down((keycode, _instant)) => {
        //             info!("key down: {}", keycode);
        //         }
        //         KeyEvent::Up((keycode, _instant)) => {
        //             info!("key up: {}", keycode);
        //         }
        //     },
        //     Err(_) => {}
        // };
    }
}

pub enum GameState {
    MainMenu(MainMenuLogicData),
    SongSelect(SongSelectLogicData),
    Playing(PlayingLogicData),
}

pub enum RenderState {
    None,
    MainMenu(MainMenuRenderData),
    SongSelect(SongSelectRenderData),
    Playing(PlayingRenderData),
}

pub enum StateTransition {
    MainMenu,
    SongSelect,
    StartSong,
    // Results,
}

pub struct StateMachine {
    current_state: GameState,
    global_data: GlobalData,
    input_rx: Receiver<KeyEvent>,
    render_input: Input<RenderState>,
}

impl StateMachine {
    fn new(
        current_state: GameState,
        global_data: GlobalData,
        input_rx: Receiver<KeyEvent>,
        render_input: Input<RenderState>,
    ) -> Self {
        Self {
            current_state,
            global_data,
            input_rx,
            render_input,
        }
    }

    fn update(&mut self) {
        let should_transition = match self.current_state {
            GameState::MainMenu(data) => main_menu::update(data),
            GameState::SongSelect(data) => song_select::update(data),
            GameState::Playing(data) => playing::update(data),
        };

        if let Some(transition) = should_transition {
            // transition away from current state
            match self.current_state {
                GameState::MainMenu(data) => main_menu::close(data),
                GameState::SongSelect(data) => song_select::close(data),
                GameState::Playing(data) => playing::close(data),
            }

            // transition to new state
            self.current_state = match transition {
                StateTransition::MainMenu => GameState::main_menu(main_menu::init()),
                StateTransition::SongSelect => GameState::song_select(song_select::init()),
                StateTransition::StartSong => GameState::playing(playing::init()),
            };
        }
    }
}
