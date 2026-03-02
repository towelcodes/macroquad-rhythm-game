use std::collections::BinaryHeap;

use macroquad::prelude::*;

use crate::{
    EntityArena, GlobalData,
    state::{GameState, StateTransition},
    tween::Tween,
};

pub struct MainMenuState {
    camera: Camera2D,
    world_arena: EntityArena,
    hud_arena: EntityArena,
    global_data: GlobalData, // tweens: BinaryHeap<Tween<f32>>, think about this later
}
impl MainMenuState {
    pub fn new(world_arena: EntityArena, hud_arena: EntityArena, global_data: GlobalData) -> Self {
        Self {
            camera: Camera2D {
                zoom: vec2(1., screen_width() / screen_height()),
                ..Default::default()
            },
            world_arena,
            hud_arena,
            global_data,
        }
    }
}
impl GameState for MainMenuState {
    async fn draw(&mut self) {
        clear_background(WHITE);
        let centre_x = screen_width() / 2.0;
        let centre_y = screen_height() / 2.0;

        // render world entities in camera space
        set_camera(&self.camera);
        draw_circle_lines(-0.15, 0.2, 0.1, 0.01, BLACK);
        draw_circle_lines(-0.15, -0.2, 0.1, 0.01, BLACK);
        draw_circle_lines(0.15, 0.2, 0.1, 0.01, BLACK);
        draw_circle_lines(0.15, -0.2, 0.1, 0.01, BLACK);

        {
            let guard = self.world_arena.read().unwrap();
            for (_idx, value) in guard.iter() {
                value.draw();
            }
        }

        // render HUD entities in screen space
        set_default_camera();
        {
            let guard = self.hud_arena.read().unwrap();
            for (_idx, value) in guard.iter() {
                value.draw();
            }
        }

        // camera debug text
        // {
        //     let mut debug_lines = self.global_data.debug_lines.lock().unwrap();
        //     *debug_lines = vec![
        //         format!("camera zoom x {:?}", cam_tween_x.get()),
        //         format!("camera zoom y {:?}", cam_tween_y.get()),
        //     ];
        // }
    }

    /// todo
    fn update(&mut self) {}

    /// todo
    fn should_transition(&self) -> Option<StateTransition> {
        None
    }

    /// todo
    fn close(self) {
        todo!()
    }
}
