use bevy::{prelude::*, render::mesh::VertexAttribute};

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

        v
    }
}

//takes a grayscale texture handle and returns a mesh with height based on the grayscale values
pub fn texture_to_mesh<T>(land_texture: &T) -> Result<Mesh, Box<dyn std::error::Error>>
    where T: HeightmapData
{
    let width = land_texture.size().0 as i32;
    let height = land_texture.size().1 as i32;

    // Define a helper to sample the underlying data
    let sample = |x, z| {
        land_texture.sample(x, z).expect("Failed to sample heightmap") as f32 / 16.0
    };

    // Generate positions
    let positions = (0..height).cartesian_product(0..width)
        .map(move |(z, x)| [x as f32, sample(x, z), z as f32])
        .collect::<Vec<_>>();

    // Generate normals
    let normals = (0..height).cartesian_product(0..width)
        .map(move |(z, x)| {
            // Sample 4 terrain points around central point
            let l = sample(x - 1, z);
            let r = sample(x + 1, z);
            let d = sample(x, z - 1);
            let u = sample(x, z + 1);

            // Calculate normal
            let norm = Vec3::new(
                l - r,
                d - u,
                2f32
            ).normalize();
            
            [norm.x(), norm.y(), norm.z()]
        })
        .collect::<Vec<_>>();

    //Generates the mesh from the information generated above using bevy's mesh generators
    let land_mesh = Mesh {
        primitive_topology: bevy::render::pipeline::PrimitiveTopology::TriangleList,
        attributes: vec![
            VertexAttribute::position(positions),
            VertexAttribute::normal(normals),
            VertexAttribute::uv(uvs(width, height)),
        ],
        indices: Some(indices(width, height)),
    };

    Ok(land_mesh)
}

fn uvs(width: i32, height: i32) -> Vec<[f32; 2]> {
    (0..height).cartesian_product(0..width)
        .map(move |(z, x)| [x as f32 / (width - 1) as f32, z as f32 / (height - 1) as f32])
        .collect::<Vec<_>>()
}

fn indices(width: i32, height: i32) -> Vec<u32> {
    let width = width as u32;
    let height = height as u32;
    
    (0..height-1).cartesian_product(0..width-1)
        .flat_map(move |(z, x)| QuadPatchGenerator::new(x + z * width, width))
        .collect::<Vec<_>>()
}
