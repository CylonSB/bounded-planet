use std::sync::Arc;
use bevy::prelude::*;
use crate::{
    networking::{
        events::{ReceiveEvent, SendEvent},
        packets::{Packet, WorldTileDataRequest, WorldTileData, StreamType}
    }
};
use super::{MeshData, TextureHeightmap, texture_to_mesh_data};

// todo(#47): stream world data
pub fn setup_world_mesh_data(mut state: ResMut<WorldTileDataState>, asset_server: Res<AssetServer>, mut textures: ResMut<Assets<Texture>>) {
    // TODO FOR US SOON: make our own asset loading system thingy for this to use (potentially other stuff)
    let land_texture_handle = asset_server
        .load_sync(&mut textures, "content/textures/CoveWorldtest.png")
        .expect("Failed to load CoveWorldtest.png");
    
    let wrap = TextureHeightmap::new(textures.get(&land_texture_handle).expect("Couldn't get texture")).expect("Couldn't wrap texture");
    state.mesh_data = Some(texture_to_mesh_data(&wrap));
}

// Notes:
// + lost `setup_world_mesh_data` system
// + generate meshes on request in `handle_world_tile_data_requests`
// + store a cache in `WorldTileDataState`
// + generate meshes async (rayon?) and stream when done

#[derive(Default)]
pub struct WorldTileDataState {
    pub event_reader: EventReader<ReceiveEvent>,
    pub mesh_data: Option<MeshData>
}

/// Handle a request from the client for a world tile
pub fn handle_world_tile_data_requests(
    mut state: ResMut<WorldTileDataState>,
    mut sender: ResMut<Events<SendEvent>>,
    receiver: ResMut<Events<ReceiveEvent>>)
{
    for evt in state.event_reader.iter(&receiver) {
        if let ReceiveEvent::ReceivedPacket { data, connection, .. } = evt {
            //todo(#46): Respect request coordinates (x, y lod)
            if let Packet::WorldTileDataRequest(WorldTileDataRequest { x: _x, y: _y, lod: _lod }) = **data {
                sender.send(
                    SendEvent::SendPacket {
                        connection: *connection,
                        stream: StreamType::WorldTileData,
                        data: Arc::new(Packet::WorldTileData(WorldTileData {
                            mesh_data: state.mesh_data.as_ref().expect("Failed to get mesh_data from WorldTileDataState").clone()
                        }))
                    }
                );
            }
        }
    }
}
