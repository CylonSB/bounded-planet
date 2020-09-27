use bevy::{prelude::*, render::mesh::VertexAttribute};

use rayon::iter::ParallelBridge;
use rayon::prelude::ParallelIterator;
use itertools::Itertools;

use super::heightmap::HeightmapData;

struct QuadPatchGenerator {
    idx: usize,
    values: [u32; 6]
}

impl QuadPatchGenerator {
    fn new(base_idx: u32, width: u32) -> QuadPatchGenerator {
        QuadPatchGenerator {
            idx: 0, 
            values: [
                base_idx,
                base_idx + width,
                base_idx + 1,
                base_idx + width,
                base_idx + width + 1,
                base_idx + 1
            ]
        }
    }
}

impl Iterator for QuadPatchGenerator {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        let v = if self.idx >= 6 {
            None
        } else {
            Some(self.values[self.idx])
        };
        self.idx += 1;

        return v;
    }
}

//takes a grayscale texture handle and returns a mesh with height based on the grayscale values
pub fn texture_to_mesh<T>(land_texture: &T) -> Result<Mesh, Box<dyn std::error::Error>>
    where T: HeightmapData
{
    let width = land_texture.size().0;
    let height = land_texture.size().1;

    // Generate UVs
    let uvs = (0..height).cartesian_product(0..width)
        .map(move |(z, x)| [x as f32 / (width - 1) as f32, z as f32 / (height - 1) as f32])
        .collect::<Vec<_>>();

    // Generate positions
    let positions = (0..height).cartesian_product(0..width)
        .map(move |(z, x)| [x as f32, land_texture.sample(x, z).expect("Failed to sample heightmap") as f32 / 16.0, z as f32])
        .collect::<Vec<_>>();

    // Generate indices
    let indices = (0..height).cartesian_product(0..width)
        .flat_map(move |(z, x)| QuadPatchGenerator::new(x + z * width, width))
        .collect::<Vec<_>>();

    // todo(#27): Generate normals
    let normals = (0..height).cartesian_product(0..width)
        .map(move |(z, x)| [0.0, 1.0, 0.0])
        .collect::<Vec<_>>();

    //Generates the mesh from the information generated above using bevy's mesh generators
    let land_mesh = Mesh {
        primitive_topology: bevy::render::pipeline::PrimitiveTopology::TriangleList,
        attributes: vec![
            VertexAttribute::position(positions),
            VertexAttribute::normal(normals),
            VertexAttribute::uv(uvs),
        ],
        indices: Some(indices),
    };

    return Ok(land_mesh);
}

//TODO: fn pub land_pipeline (Creates a render pipeline set up to use Uint32s for vertex indices)
//Note: May also include Vert shader that adds some roughness/recalculates normals and Frag shader that
//      adds some subtle color based on height or distance from camera
