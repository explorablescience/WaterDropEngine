use bevy::math::Vec3;
use wde_wgpu::vertex::Vertex;

use crate::assets::{MeshAsset, ModelBoundingBox};

pub struct CubeMesh;
impl CubeMesh {
    /// Create a new cube mesh.
    /// The cube goes from (-length/2) to (length/2) in all directions.
    /// 
    /// # Arguments
    /// 
    /// * `label` - The label for the mesh.
    /// * `length` - The length of the cube's sides.
    /// 
    /// # Returns
    /// 
    /// The cube mesh.
    pub fn from(label: &str, length: f32) -> MeshAsset {
        let half_length = length / 2.0;

        // Create vertices
        let positions = [
            // Front face
            -half_length, -half_length,  half_length,
             half_length, -half_length,  half_length,
             half_length,  half_length,  half_length,
            -half_length,  half_length,  half_length,
            // Back face
            -half_length, -half_length, -half_length,
            -half_length,  half_length, -half_length,
             half_length,  half_length, -half_length,
             half_length, -half_length, -half_length,
            // Top face
            -half_length,  half_length, -half_length,
            -half_length,  half_length,  half_length,
             half_length,  half_length,  half_length,
             half_length,  half_length, -half_length,
            // Bottom face
            -half_length, -half_length, -half_length,
             half_length, -half_length, -half_length,
             half_length, -half_length,  half_length,
            -half_length, -half_length,  half_length,
            // Right face
             half_length, -half_length, -half_length,
             half_length,  half_length, -half_length,
             half_length,  half_length,  half_length,
             half_length, -half_length,  half_length,
            // Left face
            -half_length, -half_length, -half_length,
            -half_length, -half_length,  half_length,
            -half_length,  half_length,  half_length,
            -half_length,  half_length, -half_length,
        ];
        let normals = [
            // Front face
            0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
            // Back face
            0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0,
            // Top face
            0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0,
            // Bottom face
            0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0,
            // Right face
            1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0,
            // Left face
            -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0,
        ];
        let texcoords = [
            // Front face
            0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0,
            // Back face
            1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0,
            // Top face
            0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0,
            // Bottom face
            1.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0,
            // Right face
            1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0,
            // Left face
            0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0,
        ];
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
                tangent: [0.0, 0.0, 0.0, 0.0],
            });
        }

        // Create indices
        let indices = vec![
            0, 1, 2, 0, 2, 3,       // Front face
            4, 5, 6, 4, 6, 7,       // Back face
            8, 9, 10, 8, 10, 11,    // Top face
            12, 13, 14, 12, 14, 15, // Bottom face
            16, 17, 18, 16, 18, 19, // Right face
            20, 21, 22, 20, 22, 23, // Left face
        ];

        // Create bounding box
        let bounding_box = ModelBoundingBox {
            min: Vec3::new(-half_length, -half_length, -half_length),
            max: Vec3::new( half_length,  half_length,  half_length),
        };

        MeshAsset {
            label: label.to_string(),
            vertices,
            indices,
            bounding_box,
            use_ssbo: false,
        }
    }
}
