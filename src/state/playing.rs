use std::collections::VecDeque;

use macroquad::prelude::*;

use crate::{
    EntityArena, GlobalData,
    beatmap::{Beatmap, HitObject},
    state::{GameState, StateTransition},
};

pub struct PlayingState {
    global_data: GlobalData,
    world_arena: EntityArena,
    hud_arena: EntityArena,
    beatmap: Beatmap,
    active_hit_objects: VecDeque<HitObject>,
    time: u32, // time in milliseconds
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
            active_hit_objects: VecDeque::new(), // we assume this is always sorted
            time: 0,
            bpm: bpm,
        }
    }
}
impl GameState for PlayingState {
    async fn draw(&mut self) {
        let delta = get_frame_time() * 1000.0; // ms
        let lane_speed = 20;
        let render_up_to = self.time + lane_speed * 50;

        // note: bar rendering is not implemented
        // let beat_time = (self.bpm / (60 * 60)) * 1000; // ms
        // let per_bar = self.beatmap.beats_per_bar;

        // render world entities
        {
            let guard = self.hud_arena.read().unwrap();
            for (_idx, value) in guard.iter() {
                value.draw();
            }
        }

        // render HUD entities
        {
            let guard = self.hud_arena.read().unwrap();
            for (_idx, value) in guard.iter() {
                value.draw();
            }
        }

        // process incoming messages (hits) ---
        // remove them from the heap
        // render score

        // render hit objects ---
        // - peek the next hit element and check if it is in range
        loop {
            if let Some(next) = self.beatmap.hit_objects.peek() {
                if next.0.time <= render_up_to {
                    self.active_hit_objects
                        .push_back(self.beatmap.hit_objects.pop().unwrap().0);
                    continue;
                }
            }
            break;
        }
        // - remove old hit objects
        loop {
            if let Some(last) = self.active_hit_objects.front() {
                if last.time + lane_speed * 50 < self.time {
                    self.active_hit_objects.pop_front();
                    continue;
                }
            }
            break;
        }
        // - render hit objects in the arena
        for entity in &self.active_hit_objects {
            // Calculate the position of the hit object based on its time
            let time_offset = entity.time as i32 - self.time as i32;
            let y_position = (time_offset as f32 / (lane_speed * 50) as f32) * screen_height();

            // Render the hit object at the calculated position
            let x_position =
                entity.lane as u8 as f32 * (screen_width() / 4.0) + (screen_width() / 8.0);

            draw_circle(x_position, y_position, 20.0, WHITE);
        }

        self.time += delta as u32;
    }

    fn update(&mut self) {
        todo!()
    }

    fn should_transition(&self) -> Option<StateTransition> {
        todo!()
    }

    fn close(self) {
        info!("Closing PlayingState");
    }
}
