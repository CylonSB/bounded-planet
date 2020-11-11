use bevy::prelude::{
    AppBuilder
};

use super::{id::ConnectionId, packets::{
    AuthRequest,
    AuthResponse,
    Ping,
    Pong,
    TextChat,
    WorldTileData,
    WorldTileDataRequest
}};

pub fn register_packets(app: &mut AppBuilder) {
    app.add_event::<(ConnectionId, AuthRequest)>();
    app.add_event::<(ConnectionId, AuthResponse)>();
    app.add_event::<(ConnectionId, TextChat)>();
    app.add_event::<(ConnectionId, Ping)>();
    app.add_event::<(ConnectionId, Pong)>();
    app.add_event::<(ConnectionId, WorldTileDataRequest)>();
    app.add_event::<(ConnectionId, WorldTileData)>();
}