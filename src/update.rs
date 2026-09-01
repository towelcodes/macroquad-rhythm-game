use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use crossbeam_channel::Receiver;
use macroquad::prelude::{error, info};
use triple_buffer::Input;

use crate::{
    DebugData, GlobalData,
    beatmap::Beatmap,
    data::GameConfig,
    input::KeyEvent,
    state::{
        editor::{EditorLogicData, EditorRenderData},
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
    // perform initial config load
    let config = GameConfig::load();

    // create FSM
    let mut state_machine = StateMachine::new(
        // GameState::MainMenu(main_menu::init()),
        match editor::init() {
            Ok(init_data) => GameState::Editor(init_data),
            Err(why) => {
                error!("failed to start editor: {:?}", why);
                GameState::MainMenu(main_menu::init())
            }
        },
        config,
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
    Editor(EditorLogicData),
    Playing(PlayingLogicData),
    Results(ResultsLogicData),
}

#[derive(Clone)]
pub enum RenderState {
    None,
    MainMenu(MainMenuRenderData),
    SongSelect(SongSelectRenderData),
    Editor(EditorRenderData),
    Playing(PlayingRenderData),
    Results(ResultsRenderData),
}

// TODO do this properly
pub enum StateTransition {
    MainMenu,
    SongSelect,
    Editor,
    StartBeatmap(Beatmap),
    Results(ResultsData),
    Quit,
}

pub struct StateMachine {
    current_state: GameState,
    config: GameConfig,
    global_data: GlobalData,
    input_rx: Receiver<KeyEvent>,
    render_input: Input<RenderState>,
}

impl StateMachine {
    fn new(
        current_state: GameState,
        config: GameConfig,
        global_data: GlobalData,
        input_rx: Receiver<KeyEvent>,
        render_input: Input<RenderState>,
    ) -> Self {
        Self {
            current_state,
            config,
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
            GameState::Editor(data) => editor::update(data, &mut self.render_input),
            GameState::Playing(data) => playing::update(
                data,
                self.input_rx.clone(),
                &mut self.render_input,
                &self.config,
            ),
            GameState::Results(data) => results::update(data, &mut self.render_input),
        };

        if let Some(transition) = should_transition {
            // transition away from current state
            match &mut self.current_state {
                GameState::MainMenu(data) => main_menu::close(data),
                GameState::SongSelect(data) => song_select::close(data),
                GameState::Editor(data) => editor::close(data),
                GameState::Playing(data) => playing::close(data),
                GameState::Results(data) => results::close(data),
            }

            // transition to new state
            self.current_state = match transition {
                StateTransition::MainMenu => GameState::MainMenu(main_menu::init()),
                StateTransition::SongSelect => {
                    GameState::SongSelect(song_select::init(&self.config))
                }
                StateTransition::Editor => match editor::init() {
                    Ok(init_data) => GameState::Editor(init_data),
                    Err(why) => {
                        error!("failed to start editor: {:?}", why);
                        GameState::MainMenu(main_menu::init())
                    }
                },
                StateTransition::StartBeatmap(beatmap) => {
                    match playing::init(&self.config, beatmap, self.input_rx.clone()) {
                        Ok(init_data) => GameState::Playing(init_data),
                        Err(why) => {
                            error!("failed to start playing beatmap: {:?}", why);
                            GameState::SongSelect(song_select::init(&self.config))
                        }
                    }
                }
                StateTransition::Results(data) => GameState::Results(results::init(
                    data.score,
                    data.accuracy,
                    data.judgements,
                    data.beatmap,
                )),
                StateTransition::Quit => {
                    // FIXME should quit more gracefully
                    std::process::exit(0)
                }
            };

            info!("transitioned");
        }
    }
}
