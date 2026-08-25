use std::{
    collections::HashMap,
    error::Error,
    fs::{self, File, ReadDir, read_to_string},
    io::Write,
    path::Path,
    write,
};

use macroquad::logging::{info, warn};
use serde::{Deserialize, Serialize};

use crate::{beatmap::Beatmap, input::Key};

const CONFIG_PATH: &str = "config.ron";

/// These are all functions that a key can map to.
/// A HashMap is used to represent the pairs.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyAction {
    LaneUp,
    LaneUpAlt,
    LaneDown,
    LaneDownAlt,
    Exit,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameConfig {
    pub(crate) song_folder: String, // location of the songs folder
    pub(crate) lane_speed: u32,
    pub(crate) keybinds: HashMap<Key, KeyAction>,
}
impl Default for GameConfig {
    fn default() -> Self {
        Self {
            song_folder: "songs".to_string(),
            lane_speed: 20,
            keybinds: HashMap::from([
                (Key::X, KeyAction::LaneUp),
                (Key::C, KeyAction::LaneUpAlt),
                (Key::Comma, KeyAction::LaneDown),
                (Key::Dot, KeyAction::LaneDownAlt),
                (Key::Escape, KeyAction::Exit),
            ]),
        }
    }
}

impl GameConfig {
    /// Loads the current configuration from the default path.
    /// If it does not exist, default values will be returned.
    pub fn load() -> Self {
        let path = CONFIG_PATH;
        match read_to_string(path) {
            Ok(data) => ron::from_str(&data).unwrap_or_default(),
            Err(why) => {
                warn!("could not read config, falling back to default: {:?}", why);
                Self::default()
            }
        }
    }

    /// Saves the configuration to the default path.
    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        let path = CONFIG_PATH;
        let mut output = File::create(path)?;
        let ron = ron::to_string(self)?;
        write!(output, "{}", ron)?;
        Ok(())
    }
}

// Loads the beatmaps from the specified directory
// and returns a Vec containing all of them.
pub fn load_beatmaps(path: &Path) -> Result<Vec<Beatmap>, Box<dyn Error>> {
    let paths = fs::read_dir(path)?;
    let mut beatmaps: Vec<Beatmap> = vec![];

    for path in paths {
        if let Err(why) = path {
            warn!("failed to read beatmap: {:?}", why);
            continue;
        }

        let path = path.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("ron") {
            continue;
        }

        let data = read_to_string(&path);
        if let Err(why) = data {
            warn!("failed to read beatmap {:?}: {:?}", path, why);
            continue;
        }
        let data = data.unwrap();

        match ron::from_str::<Beatmap>(&data) {
            Ok(beatmap) => {
                info!("loaded beatmap {} from {:?}", beatmap.meta.title, path);
                beatmaps.push(beatmap);
            }
            Err(why) => {
                warn!("failed to load beatmap {:?}: {:?}", path, why);
            }
        }
    }

    Ok(beatmaps)
}
