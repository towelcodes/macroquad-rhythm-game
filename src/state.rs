use std::{
    collections::BinaryHeap,
    sync::{Arc, RwLock},
};

use macroquad::prelude::*;

use crate::{
    EntityArena, GlobalData,
    beatmap::{Beatmap, HitObject},
    entity::Entity,
};

pub trait GameState {
    /// Called every frame, on the main thread.
    fn draw(&mut self);

    /// Called every update tick, on the update thread.
    fn update(&mut self);

    fn close(self);
}

pub struct PlayingState {
    global_data: GlobalData,
    world_arena: EntityArena,
    hud_arena: EntityArena,
    beatmap: Beatmap,
    entities: BinaryHeap<HitObject>,
}
impl PlayingState {
    pub fn init(
        global_data: GlobalData,
        beatmap: Beatmap,
        world_arena: EntityArena,
        hud_arena: EntityArena,
    ) -> Self {
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
