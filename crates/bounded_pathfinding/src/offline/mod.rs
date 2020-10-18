use std::{collections::HashMap, hash::Hash};

use noisy_float::prelude::*;
use pathfinding::prelude::{
    dijkstra_all,
    build_path
};

/// Precalculated information about paths from every edge face to every other edge face in a tile
#[derive(Serialize, Deserialize, Debug, Clone)]
struct EdgeToEdgePaths {
    paths: HashMap<(FaceId, FaceId), Vec<FaceId>>,
    costs: HashMap<(FaceId, FaceId), f32>,
}

/// A unique (within one tile) ID for a face
#[derive(Serialize, Deserialize, Debug, Clone)]
struct FaceId(u32);

/// A face on a navmesh
trait Face {
    /// Get all faces which neighbour this face
    fn neighbours(&self) -> &Vec<(&Self, R32)> where Self: std::marker::Sized;

    /// Get a unique ID for this face
    fn id(&self) -> FaceId;
}

/// The methods required to calculate offline pathfinding information on a world tile
trait Tile<TFace>
    where TFace: Face,
          TFace: Eq,
          TFace: Hash,
          TFace: Clone
{
    /// Get all faces which are on the border of this tile
    fn edges(&self) -> &Vec<TFace>;

    /// Calculate a path from every edge vertex to every other edge vertex
    fn calculate_edge_paths(&self) -> EdgeToEdgePaths {

        // Create a place to store the final results
        let mut costs = HashMap::new();
        let mut paths = HashMap::new();

        // Get all of the edge faces of the navmesh, these are the starting and ending points of every path
        let edge_faces = self.edges();

        for start in edge_faces {

            // For each edge face find the cheapest path to every other face
            let all = dijkstra_all(
                &start,
                |n| n.neighbours().iter().copied()
            );

            // Now extract the shortest path for each edge<->edge pair
            for end in edge_faces {
                if end.eq(start) { continue; }

                // Extract the set of faces that make up the path
                let path = build_path(&end, &all);

                // Calculate the total cost of this path
                let mut cost = R32::from_f32(0f32);
                for face in path.iter() {
                    cost += all.get(face).expect("Face was not in cost set").1;
                }

                // Store the cost of this path
                costs.insert((start.id(), end.id()), cost.raw());

                // Store the entire path
                paths.insert((start.id(), end.id()), path.iter().map(|f| f.id()).collect::<Vec::<_>>());
            }
        }

        EdgeToEdgePaths {
            costs,
            paths,
        }
    }
}