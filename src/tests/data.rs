use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serial_test::serial;

use crate::{
    beatmap::{
        Beatmap, BeatmapMeta, HitObject,
        HitObjectType::{self, Chip},
        Lane,
    },
    data::{GameConfig, KeyAction, load_beatmaps},
    input::Key,
};

const CONFIG_PATH: &str = "config.ron";

fn test_dir(rel: &str) -> PathBuf {
    env::set_current_dir(env::temp_dir()).expect("failed to set directory");
    let root = Path::new("test_tmp");
    fs::create_dir_all(root).unwrap();
    let dir = root.join(rel);
    fs::create_dir_all(&dir).unwrap();
    dir.canonicalize().unwrap()
}

#[test]
#[serial]
fn save_and_load_config() {
    // create seperate directory for testing
    let dir = test_dir("read_and_write");
    fs::create_dir_all(&dir).unwrap();
    env::set_current_dir(&dir).expect("Failed to change directory");

    // remove file if it already exists
    let _ = fs::remove_file(CONFIG_PATH);

    let config = GameConfig {
        song_folder: "my_songs".to_string(),
        lane_speed: 42,
        keybinds: [
            (Key::X, KeyAction::LaneUp),
            (Key::C, KeyAction::LaneUpAlt),
            (Key::Comma, KeyAction::LaneDown),
            (Key::Dot, KeyAction::LaneDownAlt),
            (Key::Escape, KeyAction::Exit),
        ]
        .into(),
    };

    config.save().expect("failed to save config");

    // assert the file exists
    assert!(fs::metadata(CONFIG_PATH).is_ok(), "config file not created");

    let loaded = GameConfig::load();
    assert_eq!(loaded, config);

    // restore the working directory so later tests aren't affected
    env::set_current_dir(env::temp_dir()).expect("failed to restore directory");
}

#[test]
#[serial]
fn load_beatmaps_reads_valid_ron_files() {
    // create seperate directory for testing
    let dir = test_dir("beatmaps");
    fs::create_dir_all(&dir).unwrap();
    env::set_current_dir(&dir).expect("Failed to change directory");

    // delete all files from previous runs
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        fs::remove_file(path).unwrap();
    }

    // closure to make a beatmap
    let make = |title: &str| {
        let mut hit_objects: Vec<HitObject> = vec![];
        for i in 0..50 {
            hit_objects.push(HitObject {
                time: i,
                lane: Lane::Up,
                kind: HitObjectType::Chip,
            });
        }
        Beatmap {
            meta: BeatmapMeta {
                title: title.to_string(),
                artist: "artist".to_string(),
                mapper: "mapper".to_string(),
                level: 1.0,
                level_name: "easy".to_string(),
            },
            bpm: 120,
            beats_per_bar: 4,
            audio_path: format!("{title}.ogg"),
            hit_objects,
        }
    };

    let first = make("beatmap 1");
    let second = make("beatmap 2");
    fs::write(dir.join("beatmap 1.ron"), ron::to_string(&first).unwrap()).unwrap();
    fs::write(dir.join("beatmap 2.ron"), ron::to_string(&second).unwrap()).unwrap();
    // malformed beatmap
    fs::write(
        dir.join("beatmap 3.ron"),
        b"This file is not formatted correctly",
    )
    .unwrap();
    // not a beatmap file; should be skipped
    fs::write(dir.join("notes.txt"), b"ignore me").unwrap();

    let beatmaps = load_beatmaps(&dir).expect("failed to load beatmaps");
    assert_eq!(beatmaps.len(), 2);
    assert_eq!(beatmaps[0].meta.title, "beatmap 1");
    assert_eq!(beatmaps[1].meta.title, "beatmap 2");

    env::set_current_dir(env::temp_dir()).expect("failed to restore directory");
}
