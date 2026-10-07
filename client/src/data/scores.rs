use std::error::Error;

use macroquad::logging::info;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::beatmap::{Beatmap, BeatmapMeta};
use rhythm_game_server::protocol::{JudgementsSummary, Score, User};
/// Represets a user
/// The id will be 0 if the user has no account
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OfflineUser {
    pub id: u32,
    pub name: String,
}
impl Default for OfflineUser {
    fn default() -> Self {
        Self {
            id: 0,
            name: "Anonymous".to_string(),
        }
    }
}

#[derive(Debug)]
pub struct ScoreData<'a> {
    pub beatmap: &'a Beatmap,
    pub by: OfflineUser,
    pub early_quit: bool,
    pub score: u32,
    pub accuracy: f32,
    pub judgements: JudgementsSummary,
}

pub fn init() -> Result<Connection, Box<dyn Error>> {
    let conn = Connection::open("scores.db")?;

    // check table exists
    let table_exists: bool = conn.query_row(
        "SELECT EXISTS (SELECT name FROM sqlite_master WHERE type='table' AND name='scores')",
        [],
        |row| row.get(0),
    )?;

    if !table_exists {
        // perform migration
        info!("creating scores database");
        conn.execute(
            "CREATE TABLE scores (
                    id integer PRIMARY KEY,
                    beatmap_hash blob,
                    meta_hash blob,
                    by_id integer,
                    by_name text,
                    early_quit integer,
                    score integer,
                    accuracy real,
                    perfect integer,
                    great integer,
                    okay integer,
                    bad integer,
                    miss integer
                )",
            (),
        )?;
    }

    Ok(conn)
}

pub fn get_scores_on(meta: &BeatmapMeta) -> Result<Vec<Score>, Box<dyn Error>> {
    let conn = Connection::open("scores.db")?;

    let hash = meta.hash();
    let mut stmt = conn.prepare("SELECT * FROM scores WHERE meta_hash = ?1 ORDER BY score DESC")?;
    let scores_iter = stmt.query_map([hash], |row| {
        Ok(Score {
            id: row.get(0)?,
            beatmap_hash: row.get(1)?,
            meta_hash: row.get(2)?,
            by: User {
                id: row.get(3)?,
                name: row.get(4)?,
            },
            early_quit: row.get::<_, i32>(5)? != 0,
            score: row.get(6)?,
            accuracy: row.get(7)?,
            judgements: JudgementsSummary {
                perfect: row.get(8)?,
                great: row.get(9)?,
                okay: row.get(10)?,
                bad: row.get(11)?,
                miss: row.get(12)?,
            },
        })
    })?;

    let mut scores = vec![];
    for score in scores_iter {
        scores.push(score?);
    }

    Ok(scores)
}

pub fn save_score(score: ScoreData) -> Result<(), Box<dyn Error>> {
    let conn = Connection::open("scores.db")?;

    let beatmap_hash = score.beatmap.hash();
    let meta_hash = score.beatmap.meta.hash();

    conn.execute(
        "INSERT INTO scores (
            beatmap_hash,
            meta_hash,
            by_id,
            by_name,
            early_quit,
            score,
            accuracy,
            perfect,
            great,
            okay,
            bad,
            miss
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        (
            &beatmap_hash[..],
            &meta_hash[..],
            score.by.id,
            score.by.name,
            score.early_quit as i32,
            score.score as i32,
            score.accuracy as f64,
            score.judgements.perfect as i32,
            score.judgements.great as i32,
            score.judgements.okay as i32,
            score.judgements.bad as i32,
            score.judgements.miss as i32,
        ),
    )?;
    Ok(())
}
