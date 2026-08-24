use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy)]
pub enum Lane {
    Up,
    Down,
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy)]
pub enum HitObjectType {
    Chip,
    Long,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct HitObject {
    pub time: u32, // the time in ms, from the beginning of the song
    pub lane: Lane,
    pub kind: HitObjectType, // type is a keyword
}
impl Eq for HitObject {}
impl PartialOrd for HitObject {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(&other))
    }
}
impl Ord for HitObject {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

/// Metadata associated with a Beatmap.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BeatmapMeta {
    pub title: String,
    pub artist: String,
    pub mapper: String,
    pub level: f32,
    pub level_name: String,
}

/// The full data for one level.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Beatmap {
    pub meta: BeatmapMeta,
    pub bpm: u32,
    pub beats_per_bar: u8, // will affect where the bar lines are drawn, if enabled
    pub hit_objects: Vec<HitObject>,
    pub audio_path: String,
}
