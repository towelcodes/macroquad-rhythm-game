use macroquad::prelude::*;

use crate::GlobalData;

pub trait Entity: Send + Sync {
    fn draw(&self);
}

pub struct Sprite {
    texture: Texture2D, /* should be a weak copy of the texture */
    x: f32,
    y: f32,
}
impl Entity for Sprite {
    fn draw(&self) {
        draw_texture(&self.texture, self.x, self.y, WHITE);
    }
}

pub struct FpsCounter {
    data: GlobalData,
}
impl FpsCounter {
    pub fn new(data: GlobalData) -> Self {
        Self { data }
    }
}
impl Entity for FpsCounter {
    fn draw(&self) {
        let fps = get_fps();
        let delta = get_frame_time();
        draw_text(&format!("FPS {fps}"), 10.0, 20.0, 20.0, BLACK);
        draw_text(&format!("Delta {delta}"), 10.0, 40.0, 20.0, BLACK);
        draw_text(&format!("1 - Toggle Guides"), 10.0, 60.0, 20.0, BLACK);
        draw_text(&format!("2 - Toggle HUD"), 10.0, 80.0, 20.0, BLACK);
        let debug_lines = self.data.debug_lines.lock().unwrap();
        for (i, line) in debug_lines.iter().enumerate() {
            draw_text(line, 10.0, 100.0 + (i as f32 * 20.0), 20.0, BLACK);
        }
    }
}

pub struct GridGuides;
impl Entity for GridGuides {
    fn draw(&self) {
        draw_line(0.0, 0.0, screen_width(), screen_height(), 2.0, RED);
        draw_line(0.0, screen_height(), screen_width(), 0.0, 2.0, RED);
    }
}
