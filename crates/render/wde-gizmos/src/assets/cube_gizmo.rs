use bevy::math::Vec3;
use wde_renderer::prelude::*;

pub struct CubeGizmoMesh;
impl CubeGizmoMesh {
    /// Create a new cube gizmo mesh.
    /// The cube goes from (-scale/2) to (scale/2) in all directions.
    /// 
    /// # Arguments
    /// 
    /// * `label` - The label for the mesh.
    /// * `scale` - The scale of the cube's sides.
    /// 
    /// # Returns
    /// 
    /// The cube gizmo mesh.
    pub fn from(label: &str, scale: Vec3) -> MeshAsset {
        let half_scale = scale / 2.0;

        // Create vertices
        let positions = [
            // Front face
            -half_scale.x, -half_scale.y,  half_scale.z,
             half_scale.x, -half_scale.y,  half_scale.z,
             half_scale.x,  half_scale.y,  half_scale.z,
            -half_scale.x,  half_scale.y,  half_scale.z,
            // Back face
            -half_scale.x, -half_scale.y, -half_scale.z,
             half_scale.x, -half_scale.y, -half_scale.z,
             half_scale.x,  half_scale.y, -half_scale.z,
            -half_scale.x,  half_scale.y, -half_scale.z,
        ];

        let mut vertices = Vec::new();
        for vtx in 0..positions.len() / 3 {
            let x = positions[3 * vtx];
            let y = positions[3 * vtx + 1];
            let z = positions[3 * vtx + 2];

            // Vertex
            vertices.push(Vertex {
                position: [x, y, z],
                normal: [0.0, 0.0, 0.0], // Normals are not used for gizmo
                uv: [0.0, 0.0], // UVs are not used for gizmo
                tangent: [0.0, 0.0, 0.0, 0.0],
            });
        }

        // Create indices for line list topology
        let indices = vec![
            // Front face
            0, 1, 1, 2, 2, 3, 3, 0,
            // Back face
            4, 5, 5, 6, 6, 7, 7, 4,
            // Connections
            0, 4, 1, 5, 2, 6, 3, 7,
        ];

        // Create bounding box
        let bounding_box = ModelBoundingBox {
            min: -half_scale,
            max:  half_scale,
        };

        MeshAsset {
            label: label.to_string(),
            vertices,
            indices,
            bounding_box,
        }
    }
}
