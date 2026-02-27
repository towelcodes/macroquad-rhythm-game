use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    sync::{Arc, RwLock},
};

use macroquad::prelude::*;

use crate::{
    EntityArena, GlobalData,
    beatmap::{Beatmap, HitObject},
    entity::Entity,
};

/// The game will always be in one of a fixed number of GameStates.
/// The active state will own its own data, which it can hand off
/// to the next state as necessary.
pub trait GameState {
    /// Called every frame, on the main thread.
    async fn draw(&mut self);

    /// Called every update tick, on the update thread.
    fn update(&mut self);

    fn close(self);
}

pub struct PlayingState {
    global_data: GlobalData,
    world_arena: EntityArena,
    hud_arena: EntityArena,
    beatmap: Beatmap,
    entities: BinaryHeap<Reverse<HitObject>>,
    time: u64, // time in milliseconds
    bpm: u32,  // bpm from beatmap; currently does not change, but may in future
}
impl PlayingState {
    pub fn init(
        global_data: GlobalData,
        beatmap: Beatmap,
        world_arena: EntityArena,
        hud_arena: EntityArena,
    ) -> Self {
        let bpm = beatmap.bpm;
        Self {
            global_data,
            world_arena,
            hud_arena,
            beatmap,
            entities: BinaryHeap::new(),
        }
    }
}
impl GameState for PlayingState {
    fn draw(&mut self) {}

    fn update(&mut self) {}

    fn close(self) {
        info!("Closing PlayingState");
    }
}
