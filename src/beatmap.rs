use std::collections::LinkedList;

use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lane {
    LeftUp,
    LeftDown,
    RightUp,
    RightDown,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct HitObject {
    pub beat: u32,   // the numbered beat
    pub offset: f32, // fractional offset; in beats
    pub lane: Lane,
}
impl Eq for HitObject {}
impl PartialOrd for HitObject {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(other.cmp(&self))
    }
}
impl Ord for HitObject {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match other.beat.cmp(&self.beat) {
            std::cmp::Ordering::Equal => other
                .offset
                .partial_cmp(&self.offset)
                .unwrap_or(std::cmp::Ordering::Equal),
            ord => ord,
        }
    }
}

/// Metadata associated with a Beatmap.
#[derive(Debug, Serialize, Deserialize)]
pub struct BeatmapMeta {
    pub title: String,
    pub artist: String,
    pub mapper: String,
    pub level: f32,
    pub level_name: String,
}

/// The full data for one level.
#[derive(Debug, Serialize, Deserialize)]
pub struct Beatmap {
    // audio: todo
    pub meta: BeatmapMeta,
    pub bpm: u32,
    pub hit_objects: LinkedList<HitObject>,
}
