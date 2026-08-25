use crate::beatmap::{Beatmap, BeatmapMeta, HitObject, HitObjectType, Lane};
use std::{cmp::Reverse, collections::BinaryHeap, fs::File, io::Write};

#[test]
fn create_beatmap() {
    // temporary
    let mut hit_objects: Vec<HitObject> = vec![];
    for i in 0..100 {
        hit_objects.push(HitObject {
            time: i * 500,
            lane: if i % 2 == 0 { Lane::Up } else { Lane::Down },
            kind: HitObjectType::Chip,
        });
    }
    let beatmap = Beatmap {
        meta: BeatmapMeta {
            title: "Example Beatmap".to_string(),
            artist: "Artist".to_string(),
            mapper: "mapper name".to_string(),
            level_name: "hard".to_string(),
            level: 9.5,
        },
        bpm: 200,
        beats_per_bar: 4,
        audio_path: "music.wav".to_owned(),
        hit_objects,
    };

    let serialize =
        ron::ser::to_string_pretty(&beatmap, ron::ser::PrettyConfig::default()).unwrap();
    let mut file = File::create("map2.ron").unwrap();
    file.write_all(serialize.as_bytes()).unwrap();

    println!("{serialize}");
}
