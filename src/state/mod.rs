mod main_menu;
mod playing;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};
use std::{mem, sync::mpsc::Sender};

use macroquad::prelude::*;

use crate::entity::Entity;
use crate::input::KeyEvent;
use crate::{
    EntityArena, GlobalData,
    beatmap::Beatmap,
    state::{main_menu::MainMenuState, playing::PlayingState},
};

/// All the possible states of the game,
/// used by the StateMachine.
pub enum GameStateEnum {
    // to implement:
    // MainMenu(MainMenuState)
    // SongSelect(SongSelectState)
    MainMenu(MainMenuState),
    Playing(PlayingState),
    // Paused(PausedState),
    // Results(ResultsState),
}
impl GameStateEnum {
    /// Expects to be called every frame.
    /// Must be called from the main thread.
    pub async fn draw(&mut self) {
        match self {
            Self::MainMenu(state) => state.draw().await,
            Self::Playing(state) => state.draw().await,
        }
    }

    /// Will return a StateTransition if the GameState is requesting
    /// transition to a new state. Checked every frame.
    pub fn should_transition(&self) -> Option<StateTransition> {
        match self {
            Self::MainMenu(state) => state.should_transition(),
            Self::Playing(state) => state.should_transition(),
        }
    }

    pub fn close(self) {
        match self {
            Self::MainMenu(state) => state.close(),
            Self::Playing(state) => state.close(),
        }
    }
}

/// All valid transitions between states,
/// used by the StateMachine.
pub enum StateTransition {
    MainMenu,
    SongSelect,
    Play { beatmap: Beatmap },
    Pause,
    Resume,
    Results { score: u32 },
    Quit,
}

/// The StateMachine holds the global data,
/// and owns arenas for the world and HUD.
/// It handles transition between states and hands off data.
pub struct StateMachine {
    current_state: GameStateEnum,
    global_data: GlobalData,
    world_arena: EntityArena,
    hud_arena: EntityArena,
    input_rx: Rc<RefCell<Receiver<KeyEvent>>>,
}

impl StateMachine {
    pub fn new(
        global_data: GlobalData,
        world_arena: EntityArena,
        hud_arena: EntityArena,
        input_rx: Rc<RefCell<Receiver<KeyEvent>>>,
    ) -> Self {
        Self {
            current_state: GameStateEnum::MainMenu(MainMenuState::new(
                Arc::clone(&world_arena),
                Arc::clone(&hud_arena),
                Arc::clone(&global_data),
            )),
            global_data,
            world_arena,
            hud_arena,
            input_rx,
        }
    }

    fn transition(&mut self, transition: StateTransition) {
        let new_state = match transition {
            StateTransition::MainMenu => GameStateEnum::MainMenu(MainMenuState::new(
                Arc::clone(&self.world_arena),
                Arc::clone(&self.hud_arena),
                Arc::clone(&self.global_data),
            )),
            StateTransition::Play { beatmap } => GameStateEnum::Playing(PlayingState::init(
                Arc::clone(&self.global_data),
                beatmap,
                Arc::clone(&self.world_arena),
                Arc::clone(&self.hud_arena),
                Rc::clone(&self.input_rx),
            )),
            _ => {
                todo!()
            }
        };

        let old_state = mem::replace(&mut self.current_state, new_state);
        old_state.close();
    }

    /// Called every frame, on the main thread.
    pub async fn draw(&mut self) {
        self.current_state.draw().await;

        // check for transition
        if let Some(transition) = self.current_state.should_transition() {
            self.transition(transition);
        }
    }

    /// Called every update tick, on the update thread.
    pub fn update(&self) {
        self.current_state.update();
    }
}

/// The game will always be in one of a fixed number of GameStates.
/// The active state will own its own data, which it can hand off
/// to the next state as necessary.
pub trait GameState {
    /// Called every frame, on the main thread.
    async fn draw(&mut self);

    /// Will return a StateTransition if the GameState is requesting
    /// transition to a new state. Checked every frame.
    fn should_transition(&self) -> Option<StateTransition>;

    fn close(self);
}
