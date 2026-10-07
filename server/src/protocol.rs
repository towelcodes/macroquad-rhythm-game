use std::{
    error::Error,
    io::{Read, Write},
};

use serde::{Deserialize, Serialize};

// packet structure:
// [ len (4 bytes) ] [ messagepack content (serde) ]

#[repr(u8)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NetError {
    Malformed,
    TooShort(u32),
    Other(String),
}

#[repr(u8)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerboundPacket {
    Hello,
    Ping,
    Pong,
    Login { username: String, password: String },
    Register { username: String, password: String },
    SubmitScore { token: [u8; 32], score: Score },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct JudgementsSummary {
    pub perfect: u32,
    pub great: u32,
    pub okay: u32,
    pub bad: u32,
    pub miss: u32,
}

/// Represents a score in the database
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

pub fn recv<T: for<'de> Deserialize<'de>>(stream: &mut impl Read) -> Result<T, Box<dyn Error>> {
    // read length
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf);
    println!("recv: len {}", len);
    let mut buf = vec![0u8; len as usize];
    stream.read_exact(&mut buf)?;
    Ok(rmp_serde::from_slice(&buf)?)
}

pub fn send<T: Serialize>(payload: T, stream: &mut impl Write) -> Result<(), Box<dyn Error>> {
    let payload = rmp_serde::to_vec(&payload)?;
    let len = payload.len() as u32;
    let len_bytes = len.to_be_bytes();
    stream.write_all(&len_bytes)?;
    stream.write_all(&payload)?;
    Ok(())
}
