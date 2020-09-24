use bevy::{prelude::*, render::mesh::VertexAttribute};

//takes a grayscale texture handle and returns a mesh with height based on the grayscale values
pub fn texture_to_mesh(
    textures: ResMut<Assets<Texture>>,
    land_texture_handle: Handle<Texture>,
) -> Option<Mesh> {
    //gets a reference to the data needed
    let land_texture = textures.get(&land_texture_handle)?;

    //prepares the Vecs with the capacity they need.
    let mut land_positions =
        Vec::with_capacity((land_texture.size.x() * land_texture.size.y()) as usize);
    let mut land_normals =
        Vec::with_capacity((land_texture.size.x() * land_texture.size.y()) as usize);
    let mut land_uvs = Vec::with_capacity((land_texture.size.x() * land_texture.size.y()) as usize);

    let mut land_indices = Vec::with_capacity(
        ((land_texture.size.x() - 1.0) * (land_texture.size.y() - 1.0)) as usize,
    );

    //Loop through all pixels in the texture in their vertex order
    for z in 0..(land_texture.size.y() as i32) {
        for x in 0..land_texture.size.x() as i32 {
            //index number for the current vertex
            let i: u32 = x as u32 + z as u32 * land_texture.size.y() as u32;

            //pushes the current Vertex's VertexAttributes to the correct Vecs.
            //TODO(#28): Use sampler to get mipped height data, rather than using direct data
            land_positions.push([
                x as f32,
                land_texture.data[i as usize] as f32 / 16.0,
                z as f32,
            ]);

            //TODO(#27): Generate normals based on texture data
            land_normals.push([0.0, 1.0, 0.0]);

            land_uvs.push([
                x as f32 / (land_texture.size.x() - 1.0),
                z as f32 / (land_texture.size.y() - 1.0),
            ]);

            //For vertexes that aren't on the end edges, input 2 triangles into the triangle list, clockwise
            if x != (land_texture.size.x() - 1.0) as i32
                && z != (land_texture.size.y() - 1.0) as i32
            {
                land_indices.push(i);
                land_indices.push(i + land_texture.size.x() as u32);
                land_indices.push(i + land_texture.size.x() as u32 + 1);

                land_indices.push(i);
                land_indices.push(i + land_texture.size.x() as u32 + 1);
                land_indices.push(i + 1);
            }
        }
    }

    //Generates the mesh from the information generated above using bevy's mesh generators
    let land_mesh = Mesh {
        primitive_topology: bevy::render::pipeline::PrimitiveTopology::TriangleList,
        attributes: vec![
            VertexAttribute::position(land_positions),
            VertexAttribute::normal(land_normals),
            VertexAttribute::uv(land_uvs),
        ],
        indices: Some(land_indices),
    };

    return Some(land_mesh);
}

//TODO(#26): fn pub land_pipeline (Creates a render pipeline set up to use Uint32s for vertex indices)
//Note: May also include Vert shader that adds some roughness/recalculates normals and Frag shader that
//      adds some subtle color based on height or distance from camera
