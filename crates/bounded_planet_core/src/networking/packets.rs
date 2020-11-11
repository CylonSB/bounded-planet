use std::{sync::Arc, time::SystemTime};
use serde::{Deserialize, Serialize};

use crate::land::MeshData;

/// Uniquely identifies a single unidirectional stream of data within a single network connection
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum StreamType {
    TextChat,
    PingPong,
    WorldTileData
}

/// Enum of all packets in the network protocol
// REMEMBER: After adding a new variant here, also go and add an event with the correct type in dispatch.rs
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Packet {
    AuthRequest(AuthRequest),
    AuthResponse(AuthResponse),
    TextChat(TextChat),
    Ping(Ping),
    Pong(Pong),
    WorldTileDataRequest(WorldTileDataRequest),
    WorldTileData(WorldTileData)
}

/// Ping packet, expects a returned "Pong" response
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Ping {
    pub timestamp: u64,
}

impl Default for Ping {
    fn default() -> Self {
        Ping {
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("System time is before unix epoch")
                .as_millis() as u64
        }
    }
}

/// Pong packet, contains the timestamp of the Ping packet it is responding to
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pong {
    pub timestamp: u64,
}

/// Authentication information sent from the client to the server
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AuthResponse {
    Ok,
    IncorrectUsername,
    IncorrectPassword,
}

/// A text chat message
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TextChat {
    pub index: u64,
    pub message: String,
}

/// World tile data request packet, sent by the client to the server
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WorldTileDataRequest {
    pub x: u32,
    pub y: u32,
    pub lod: u8,
}

/// World tile data packet, requested by the client
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WorldTileData {
    pub mesh_data: Arc<MeshData>
}
