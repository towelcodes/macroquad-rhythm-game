use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use crossbeam_channel::{Receiver, Sender};
use macroquad::{
    color::{Color, RED},
    logging::warn,
    prelude::{error, info},
};
use triple_buffer::Input;

use crate::{
    DebugData, GlobalData, Notification,
    beatmap::Beatmap,
    data::GameConfig,
    input::KeyEvent,
    net::{
        self,
        overlay::{NetOverlay, NetOverlayRenderData},
    },
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
    notify_tx: Sender<Notification>,
    render_input: Input<RenderState>,
    debug_input: &mut Input<DebugData>,
    net_render_input: Input<NetOverlayRenderData>,
    net_req_tx: Sender<(u32, net::NetRequest)>,
    net_res_rx: Receiver<(u32, net::NetResponse)>,
) {
    // perform initial config load
    let config = GameConfig::load();

    // create FSM
    let mut state_machine = StateMachine::new(
        GameState::MainMenu(main_menu::init()),
        config,
        global_data,
        input_rx,
        notify_tx,
        render_input,
    );

    // create net overlay
    let mut net_overlay = NetOverlay::new(net_req_tx, net_res_rx, net_render_input);

    let target = Duration::from_secs_f32(1.0 / 500.0); // 500hz
    let mut last = Instant::now();

    info!("started update thread");
    loop {
        state_machine.update();
        net_overlay.update();

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
    notify_tx: Sender<Notification>,
    render_input: Input<RenderState>,
}

impl StateMachine {
    fn new(
        current_state: GameState,
        config: GameConfig,
        global_data: GlobalData,
        input_rx: Receiver<KeyEvent>,
        notify_tx: Sender<Notification>,
        render_input: Input<RenderState>,
    ) -> Self {
        Self {
            current_state,
            config,
            global_data,
            input_rx,
            notify_tx,
            render_input,
        }
    }

    fn update(&mut self) {
        let should_transition = match &mut self.current_state {
            GameState::MainMenu(data) => {
                main_menu::update(data, &mut self.config, &mut self.render_input)
            }
            GameState::SongSelect(data) => song_select::update(
                data,
                self.input_rx.clone(),
                &self.config.keybinds,
                &mut self.render_input,
            ),
            GameState::Editor(data) => editor::update(data, &mut self.render_input),
            GameState::Playing(data) => playing::update(
                data,
                self.input_rx.clone(),
                &mut self.render_input,
                &self.config,
            ),
            GameState::Results(data) => results::update(
                data,
                &mut self.render_input,
                self.input_rx.clone(),
                &self.config.keybinds,
            ),
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
                    GameState::SongSelect(song_select::init(&self.config, self.notify_tx.clone()))
                }
                StateTransition::Editor => {
                    match editor::init(&self.config, self.input_rx.clone()) {
                        Ok(init_data) => GameState::Editor(init_data),
                        Err(why) => {
                            if let Err(why2) = self.notify_tx.send(Notification {
                                color: RED,
                                content: "Failed to start the editor. Details have been logged."
                                    .into(),
                            }) {
                                warn!("failed to send error notification {:?}", why2);
                            }
                            error!("failed to start editor: {:?}", why);
                            GameState::MainMenu(main_menu::init())
                        }
                    }
                }
                StateTransition::StartBeatmap(beatmap) => {
                    match playing::init(&self.config, beatmap, self.input_rx.clone()) {
                        Ok(init_data) => GameState::Playing(init_data),
                        Err(why) => {
                            if let Err(why2) = self.notify_tx.send(Notification {
                                color: RED,
                                content: "Failed to start the beatmap. Is the audio available?"
                                    .into(),
                            }) {
                                warn!("failed to send error notification {:?}", why2);
                            }
                            error!("failed to start playing beatmap: {:?}", why);
                            GameState::SongSelect(song_select::init(
                                &self.config,
                                self.notify_tx.clone(),
                            ))
                        }
                    }
                }
                StateTransition::Results(data) => GameState::Results(results::init(
                    &self.config,
                    self.notify_tx.clone(),
                    data.score,
                    data.accuracy,
                    data.judgements,
                    data.early_quit,
                    data.beatmap,
                )),
                StateTransition::Quit => {
                    // FIXME should quit more gracefully
                    // save config!
                    if let Err(why) = self.config.save() {
                        error!("failed to save config: {:?}", why);
                    }
                    info!("saved config");
                    std::process::exit(0)
                }
            };

            info!("transitioned");
        }
    }
}
