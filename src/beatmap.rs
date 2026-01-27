use std::collections::LinkedList;

#[derive(Debug, PartialEq, PartialOrd, Eq)]
pub struct HitObject {
    timestamp: u32,
    lane: u8,
}
impl Ord for HitObject {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.timestamp.cmp(&self.timestamp)
    }
}

pub struct Beatmap {
    hit_objects: LinkedList<HitObject>,
}
