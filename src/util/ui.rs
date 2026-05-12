use macroquad::{prelude::*, ui::root_ui};

pub enum AnchorPoint {
    TopLeft,
    BottomLeft,
    Centre,
}

fn calculate_position(p: Vec2, anchor: AnchorPoint, size: Vec2) -> Vec2 {
    let (w, h) = (screen_width(), screen_height());

    let offset = match anchor {
        AnchorPoint::Centre => Vec2::new(size.x / 2., size.y / 2.),
        AnchorPoint::TopLeft => Vec2::new(0., size.y),
        AnchorPoint::BottomLeft => Vec2::new(0., 0.),
    };

    Vec2::new(p.x * w, p.y * h) - offset
}

pub fn button<P>(position: P, content: &str) -> bool
where
    P: Into<Option<(Vec2, AnchorPoint)>>,
{
    // TODO: this does not account for padding in the skin
    let p = position.into();
    if p.is_none() {
        return root_ui().button(None, content);
    }

    let (p, anchor) = p.unwrap();
    let size = root_ui().calc_size(content);
    let vec = calculate_position(p, anchor, size);

    root_ui().button(vec, content)
}

/// Places a label at the specified relative position
pub fn label<P>(position: P, content: &str)
where
    P: Into<Option<(Vec2, AnchorPoint)>>,
{
    let p = position.into();
    if p.is_none() {
        return root_ui().label(None, content);
    }

    let (p, anchor) = p.unwrap();
    let size = root_ui().calc_size(content);
    let vec = calculate_position(p, anchor, size);

    root_ui().label(vec, content)
}
