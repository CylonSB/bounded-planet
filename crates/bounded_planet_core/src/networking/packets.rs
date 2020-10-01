use serde::{Deserialize, Serialize};

/// Uniquely identifies a single unidirectional stream of data within a single network connection
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum StreamType {
    TextChat,
    Ping,
}

/// Enum of all packets in the network protocol
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Packet {
    AuthRequest(AuthRequest),
    AuthResponse(AuthResponse),
    TextChat(TextChat),
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
