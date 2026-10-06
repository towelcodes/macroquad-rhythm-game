use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct BeatmapMeta {
    pub title: String,
    pub artist: String,
    pub mapper: String,
    pub level: f32, // TODO change to u8
    pub level_name: String,
}
impl BeatmapMeta {
    /// Get a sha256 of the BeatmapMeta
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{}\0{}\0{}\0{}\0",
            self.title, self.artist, self.mapper, self.level_name,
        ));
        hasher.update(self.level.to_bits().to_be_bytes());
        hasher.finalize().into()
    }
}

/// The full data for one level.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Beatmap {
    pub meta: BeatmapMeta,
    pub bpm: u32,
    pub hit_objects: Vec<HitObject>,
    pub audio_path: String,
}
impl Default for Beatmap {
    fn default() -> Self {
        Self {
            meta: BeatmapMeta::default(),
            bpm: 120,
            hit_objects: Vec::new(),
            audio_path: String::new(),
        }
    }
}
impl Beatmap {
    /// Get a sha256 of the Beatmap
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.meta.hash());

        // sort hit objects
        let mut sorted: Vec<&HitObject> = self.hit_objects.iter().collect();
        sorted.sort_by_key(|h| (h.time, h.lane as u8, h.kind as u8));
        for h in sorted {
            hasher.update(format!("{:?}\0{:?}\0{:?}", h.time, h.kind, h.lane));
        }

        hasher.finalize().into()
    }
}
