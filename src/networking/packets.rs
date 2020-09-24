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
