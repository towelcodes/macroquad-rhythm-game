use crossbeam_channel::{Receiver, Sender};
use macroquad::{
    prelude::*,
    ui::{Skin, root_ui},
};
use triple_buffer::Input;

use crate::{
    AssetStore,
    beatmap::{Beatmap, BeatmapMeta},
    state::playing::Judgement,
    update::{RenderState, StateTransition},
    util::ui::{self, AnchorPoint},
};

enum UiEvent {
    MainMenu,
    Retry,
}

/// Payload carried by the `StateTransition::Results` transition.
pub struct ResultsData {
    pub score: u32,
    pub accuracy: f32,
    pub judgements: Vec<(Judgement, u32)>,
    pub beatmap: Beatmap,
}

pub struct ResultsLogicData {
    score: u32,
    accuracy: f32,
    judgements: Vec<(Judgement, u32)>,
    beatmap: Beatmap,
    ui_tx: Sender<UiEvent>,
    ui_rx: Receiver<UiEvent>,
}

#[derive(Clone)]
pub struct ResultsRenderData {
    score: u32,
    accuracy: f32,
    perfect: u32,
    great: u32,
    okay: u32,
    bad: u32,
    miss: u32,
    meta: BeatmapMeta,
    ui_events_sender: Sender<UiEvent>,
}

pub fn init(
    score: u32,
    accuracy: f32,
    judgements: Vec<(Judgement, u32)>,
    beatmap: Beatmap,
) -> ResultsLogicData {
    let (ui_tx, ui_rx) = crossbeam_channel::unbounded();
    ResultsLogicData {
        score,
        accuracy,
        judgements,
        beatmap,
        ui_tx,
        ui_rx,
    }
}

/// Run when closing the state (blocks update thread)
pub fn close(_data: &ResultsLogicData) {}

pub fn update(
    data: &mut ResultsLogicData,
    render_input: &mut Input<RenderState>,
) -> Option<StateTransition> {
    for event in data.ui_rx.try_iter() {
        match event {
            UiEvent::MainMenu => {
                return Some(StateTransition::MainMenu);
            }
            UiEvent::Retry => {
                return Some(StateTransition::StartBeatmap(data.beatmap.clone()));
            }
        }
    }

    // count judgements
    let (mut perfect, mut great, mut okay, mut bad, mut miss) = (0, 0, 0, 0, 0);
    for (judgement, _) in data.judgements.iter() {
        match judgement {
            Judgement::Perfect(_) => perfect += 1,
            Judgement::Great(_) => great += 1,
            Judgement::Ok(_) => okay += 1,
            Judgement::Bad(_) => bad += 1,
            Judgement::Miss(_) => miss += 1,
        }
    }

    render_input.write(RenderState::Results(ResultsRenderData {
        score: data.score,
        accuracy: data.accuracy,
        perfect,
        great,
        okay,
        bad,
        miss,
        meta: data.beatmap.meta.clone(),
        ui_events_sender: data.ui_tx.clone(),
    }));

    None
}

pub async fn render(data: &ResultsRenderData, _assets: &AssetStore) {
    clear_background(WHITE);

    // set the UI skin
    let label_style = root_ui().style_builder().font_size(24).build();
    let skin = Skin {
        label_style,
        ..root_ui().default_skin()
    };
    root_ui().push_skin(&skin);

    // title
    ui::label(
        (vec2(0.5, 0.1), AnchorPoint::Centre),
        &format!("{}", data.meta.title),
    );
    ui::label(
        (vec2(0.5, 0.16), AnchorPoint::Centre),
        &format!("by {}", data.meta.artist),
    );

    // score and accuracy
    ui::label(
        (vec2(0.5, 0.28), AnchorPoint::Centre),
        &format!("Score: {:07}", data.score),
    );
    ui::label(
        (vec2(0.5, 0.34), AnchorPoint::Centre),
        &format!("Accuracy: {:.2}%", data.accuracy * 100.0),
    );

    // judgement counts
    ui::label((vec2(0.5, 0.42), AnchorPoint::Centre), "Judgements");
    ui::label(
        (vec2(0.5, 0.48), AnchorPoint::Centre),
        &format!(
            "Perfect: {}   Great: {}   Ok: {}",
            data.perfect, data.great, data.okay
        ),
    );
    ui::label(
        (vec2(0.5, 0.54), AnchorPoint::Centre),
        &format!("Bad: {}   Miss: {}", data.bad, data.miss),
    );

    // buttons
    if ui::button((vec2(0.4, 0.75), AnchorPoint::Centre), "Retry") {
        if let Err(why) = data.ui_events_sender.send(UiEvent::Retry) {
            warn!("error sending ui event: {why:?}");
        }
    }
    if ui::button((vec2(0.6, 0.75), AnchorPoint::Centre), "Main Menu") {
        if let Err(why) = data.ui_events_sender.send(UiEvent::MainMenu) {
            warn!("error sending ui event: {why:?}");
        }
    }

    root_ui().pop_skin();
}
