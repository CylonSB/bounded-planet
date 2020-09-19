use bevy::{
    prelude::*,
    render::mesh::VertexAttribute,
    prelude};



//takes a grayscale texture handle and returns a mesh with height based on the grayscale values
pub fn texture_to_mesh (
    textures: ResMut<Assets<Texture>>,
    land_texture_handle: prelude::Handle<prelude::Texture>,
) -> bevy::prelude::Mesh {

    //gets a copy of the 
    let land_texture = textures.get(&land_texture_handle).unwrap();
 
    let mut land_positions = Vec::with_capacity((land_texture.size.x() * land_texture.size.y()) as usize);
    let mut land_normals = Vec::with_capacity((land_texture.size.x() * land_texture.size.y()) as usize);
    let mut land_uvs = Vec::with_capacity((land_texture.size.x() * land_texture.size.y()) as usize);

    let mut land_indices = Vec::with_capacity(((land_texture.size.x() - 1.0) * (land_texture.size.y() - 1.0)) as usize);

    for z in 0..(land_texture.size.y() as i32) {
        for x in 0..land_texture.size.x() as i32 {
            
            let i: u32 = x as u32 + z as u32 * land_texture.size.y() as u32;
            
            land_positions.push([x as f32, land_texture.data[i as usize] as f32 / 16.0 , z as f32]);
            land_normals.push([0.0, 1.0, 0.0]);
            land_uvs.push([x as f32 / land_texture.size.x(), z as f32 / land_texture.size.y()]);


            if x != (land_texture.size.x() - 1.0) as i32 && z != (land_texture.size.y() - 1.0) as i32 {
                land_indices.push(i);
                land_indices.push(i + land_texture.size.x() as u32); 
                land_indices.push(i + land_texture.size.x() as u32 + 1);

                land_indices.push(i);
                land_indices.push(i + land_texture.size.x() as u32 + 1);
                land_indices.push(i + 1);
            }
        }
    }

    let land_mesh = Mesh {
        primitive_topology: bevy::render::pipeline::PrimitiveTopology::TriangleList,
        attributes: vec![
            VertexAttribute::position(land_positions),
            VertexAttribute::normal(land_normals),
            VertexAttribute::uv(land_uvs),
        ],
        indices: Some(land_indices),
    };

    return land_mesh;
}