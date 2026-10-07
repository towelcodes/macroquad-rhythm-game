use serde::{Deserialize, Serialize};

// packet structure:
// [ len (4 bytes) ] [ messagepack content (serde) ]

#[repr(u8)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetError {
    Malformed,
    TooShort(u32),
    Other(String),
}

#[repr(u8)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerboundPacket {
    Hello,
    Ping,
    Pong,
    Login { username: String, password: String },
    Register { username: String, password: String },
    SubmitScore { token: [u8; 32], score: Score },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientboundPacket {
    Error(NetError),
    Ok,
    Ping,
    Pong,
    TokenInvalid,
    AuthInvalid,
    AuthValid([u8; 32]),
}

/// Represets a user
/// The id will be 0 if the user has no auth
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct JudgementsSummary {
    pub perfect: u32,
    pub great: u32,
    pub okay: u32,
    pub bad: u32,
    pub miss: u32,
}

/// Represents a score in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Score {
    pub id: i32,
    /// sha256 of the beatmap data
    pub beatmap_hash: [u8; 32],
    /// sha256 of the beatmap meta; to match older versions
    pub meta_hash: [u8; 32],
    pub by: User,
    pub early_quit: bool,
    pub score: u32,
    pub accuracy: f32,
    pub judgements: JudgementsSummary,
}
