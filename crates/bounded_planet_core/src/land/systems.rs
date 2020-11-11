use std::sync::Arc;
use bevy::prelude::*;
use crate::networking::{
    id::ConnectionId,
    events::SendEvent,
    packets::{
        Packet,
        WorldTileData,
        WorldTileDataRequest
    }
};
use super::MeshData;

/// Loads the world mesh and stores it
pub fn setup_world_mesh_data(mut state: ResMut<WorldTileDataState>) {
    //todo(#47):
    // - Load world data on demand, instead of ahead of time like this
    // - Use the asset server to load content?
    // - Store a cache in `WorldTileDataState` (or load through asset server and unload)?

    let mesh_data = rmp_serde::from_read::<_, MeshData>(
        flate2::read::ZlibDecoder::new(
            std::fs::File::open("content/worlds/CoveWorldtest.bpmesh").expect("Failed to load 'content/worlds/CoveWorldtest.bpmesh'")
        )
    ).expect("Failed to deserialized 'content/worlds/CoveWorldtest.bpmesh'");
    state.mesh_data = Some(Arc::new(mesh_data));
}

#[derive(Default)]
pub struct WorldTileDataState {
    pub event_reader: EventReader<(ConnectionId, WorldTileDataRequest)>,
    pub mesh_data: Option<Arc<MeshData>>
}

/// Handle a request from the client for a world tile
pub fn handle_world_tile_data_requests(
    mut state: ResMut<WorldTileDataState>,
    mut sender: ResMut<Events<SendEvent>>,
    receiver: ResMut<Events<(ConnectionId, WorldTileDataRequest)>>)
{
    //todo(#46): Respect request coordinates (x, y lod)
    for (conn, _) in state.event_reader.iter(&receiver) {
        sender.send(
            SendEvent::TransferPacket {
                connection: *conn,
                data: Packet::WorldTileData(WorldTileData {
                    mesh_data: state.mesh_data.as_ref().expect("Failed to get mesh_data from WorldTileDataState").clone()
                })
            }
        );
    }
}
