use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use crossbeam_channel::Receiver;
use macroquad::prelude::info;
use triple_buffer::Input;

use crate::{
    DebugData, GlobalData,
    beatmap::Beatmap,
    input::KeyEvent,
    state::{
        main_menu::{MainMenuLogicData, MainMenuRenderData},
        playing::{PlayingLogicData, PlayingRenderData},
        results::{ResultsData, ResultsLogicData, ResultsRenderData},
        song_select::{SongSelectLogicData, SongSelectRenderData},
        *,
    },
};

pub fn start_update_thread(
    global_data: GlobalData,
    input_rx: Receiver<KeyEvent>,
    render_input: Input<RenderState>,
    debug_input: &mut Input<DebugData>,
) {
    // create FSM
    let mut state_machine = StateMachine::new(
        GameState::MainMenu(main_menu::init()),
        global_data,
        input_rx,
        render_input,
    );

    let target = Duration::from_secs_f32(1.0 / 500.0); // 500hz
    let mut last = Instant::now();

    info!("started update thread");
    loop {
        state_machine.update();

        debug_input.write(DebugData {
            show: true,
            update_delta: Instant::now().duration_since(last).as_millis(),
            update_target: target.as_millis(),
        });

        // avoid pinning the cpu
        target
            .checked_sub(last.elapsed())
            .map(|remaining| thread::sleep(remaining))
            .unwrap_or_default();
        last = Instant::now();
    }
}

pub enum GameState {
    MainMenu(MainMenuLogicData),
    SongSelect(SongSelectLogicData),
    Playing(PlayingLogicData),
    Results(ResultsLogicData),
}

#[derive(Clone)]
pub enum RenderState {
    None,
    MainMenu(MainMenuRenderData),
    SongSelect(SongSelectRenderData),
    Playing(PlayingRenderData),
    Results(ResultsRenderData),
}

// TODO do this properly
pub enum StateTransition {
    MainMenu,
    SongSelect,
    StartBeatmap(Beatmap),
    Results(ResultsData),
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
        let should_transition = match &mut self.current_state {
            GameState::MainMenu(data) => main_menu::update(
                data,
                Arc::clone(&self.global_data),
                self.input_rx.clone(),
                &mut self.render_input,
            ),
            GameState::SongSelect(data) => song_select::update(
                data,
                Arc::clone(&self.global_data),
                self.input_rx.clone(),
                &mut self.render_input,
            ),
            GameState::Playing(data) => {
                playing::update(data, self.input_rx.clone(), &mut self.render_input)
            }
            GameState::Results(data) => results::update(data, &mut self.render_input),
        };

        if let Some(transition) = should_transition {
            // transition away from current state
            match &mut self.current_state {
                GameState::MainMenu(data) => main_menu::close(data),
                GameState::SongSelect(data) => song_select::close(data),
                GameState::Playing(data) => playing::close(data),
                GameState::Results(data) => results::close(data),
            }

            // transition to new state
            self.current_state = match transition {
                StateTransition::MainMenu => GameState::MainMenu(main_menu::init()),
                StateTransition::SongSelect => GameState::SongSelect(song_select::init()),
                StateTransition::StartBeatmap(beatmap) => {
                    GameState::Playing(playing::init(beatmap, self.input_rx.clone()))
                }
                StateTransition::Results(data) => GameState::Results(results::init(
                    data.score,
                    data.accuracy,
                    data.judgements,
                    data.beatmap,
                )),
            };

            info!("transitioned");
        }
    }
}
