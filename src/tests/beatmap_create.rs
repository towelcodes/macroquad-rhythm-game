use crate::beatmap::{Beatmap, BeatmapMeta, HitObject, HitObjectType, Lane};
use std::{cmp::Reverse, collections::BinaryHeap, fs::File, io::Write};

// #[test]
// fn create_beatmap() {
//     let hit_objects = BinaryHeap::from([
//         Reverse(HitObject {
//             beat: 0,
//             offset: 0.0,
//             lane: Lane::RightUp,
//             ttype: HitObjectType::Chip,
//         }),
//         Reverse(HitObject {
//             beat: 1,
//             offset: 0.0,
//             lane: Lane::RightDown,
//             ttype: HitObjectType::Chip,
//         }),
//     ]);

//     let meta = BeatmapMeta {
//         title: "example".to_string(),
//         artist: "camellia".to_string(),
//         mapper: "teatowel".to_string(),
//         level: 5.0,
//         level_name: "insane".to_string(),
//     };

//     let beatmap = Beatmap {
//         meta,
//         hit_objects,
//         bpm: 120,
//         time_signature: (4, 1.),
//     };

//     let serialize =
//         ron::ser::to_string_pretty(&beatmap, ron::ser::PrettyConfig::default()).unwrap();
//     let mut file = File::create("map.ron").unwrap();
//     file.write_all(serialize.as_bytes()).unwrap();

//     println!("{serialize}");
// }
