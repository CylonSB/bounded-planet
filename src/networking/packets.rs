use serde::{Deserialize, Serialize};

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

/// Authentication response sent form the server to the client
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthResponse {
    pub accepted: bool,
    pub message: Option<String>,
}

/// A text chat message
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TextChat {
    pub index: u64,
    pub message: String,
}
