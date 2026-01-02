use bevy::math::Vec3;
use wde_wgpu::vertex::Vertex;

use crate::assets::{MeshAsset, ModelBoundingBox};

pub struct PlaneMesh;
impl PlaneMesh {
    /// Create a new plane mesh.
    /// 
    /// # Arguments
    /// 
    /// * `size` - The size in the x and y direction.
    ///   The plane will be centered at the origin.
    /// 
    /// # Returns
    /// 
    /// The plane mesh.
    pub fn from(label: &str, size: [f32; 2]) -> MeshAsset {
        let half_size = [size[0] / 2.0, size[1] / 2.0];

        // Create vertices
        let positions = [
            -half_size[0], 0.0, -half_size[1],
             half_size[0], 0.0, -half_size[1],
             half_size[0], 0.0,  half_size[1],
            -half_size[0], 0.0,  half_size[1],
        ];
        let normals  = [0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0];
        let texcoords = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
        let mut vertices = Vec::new();
        for vtx in 0..positions.len() / 3 {
            let x = positions[3 * vtx];
            let y = positions[3 * vtx + 1];
            let z = positions[3 * vtx + 2];

            // Normals
            let nx = normals[3 * vtx];
            let ny = normals[3 * vtx + 1];
            let nz = normals[3 * vtx + 2];

            // UVs
            let u = texcoords[2 * vtx];
            let v = texcoords[2 * vtx + 1];

            // Vertex
            vertices.push(Vertex {
                position: [x, y, z],
                normal: [nx, ny, nz],
                uv: [u, v],
            });
        }

        // Create indices
        let indices = vec![0, 2, 1, 0, 3, 2];

        // Create bounding box
        let bounding_box = ModelBoundingBox {
            min: Vec3::new(-half_size[0], 0.0, -half_size[1]),
            max: Vec3::new( half_size[0], 0.0,  half_size[1]),
        };

        MeshAsset {
            label: label.to_string(),
            vertices,
            indices,
            bounding_box,
        }
    }
}

