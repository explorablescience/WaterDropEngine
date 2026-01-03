use bevy::math::Vec3;
use wde_wgpu::vertex::Vertex;

use crate::assets::{MeshAsset, ModelBoundingBox};

pub struct PlaneMesh;
impl PlaneMesh {
    /// Create a new plane mesh.
    /// 
    /// # Arguments
    /// 
    /// * `label` - The label for this mesh asset.
    /// * `size` - The size in the u and v direction.
    ///   The plane will be centered at the origin.
    /// * `subdivisions` - Number of subdivisions per axis (minimum 1).
    /// * `normal` - The normal direction of the plane.
    /// 
    /// # Returns
    /// 
    /// The plane mesh.
    pub fn from(label: &str, subdivisions: u32, normal: Vec3) -> MeshAsset {
        let subdivisions = subdivisions.max(1);
        let half_size = [0.5, 0.5];

        // Normalize the normal vector
        let normal = normal.normalize();

        // Compute tangent and bitangent vectors to form a coordinate system
        let (tangent, bitangent) = compute_tangent_bitangent(normal);

        // Generate vertices
        let mut vertices = Vec::new();
        let mut min_bounds = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut max_bounds = Vec3::new(f32::MIN, f32::MIN, f32::MIN);

        for i in 0..=subdivisions {
            for j in 0..=subdivisions {
                let u = (i as f32) / (subdivisions as f32);
                let v = (j as f32) / (subdivisions as f32);

                // Position in local plane space [-half_size to +half_size]
                let local_x = -half_size[0] + u;
                let local_y = -half_size[1] + v;

                // Transform to world space using tangent/bitangent basis
                let position = tangent * local_x + bitangent * local_y;

                // Update bounding box
                min_bounds = min_bounds.min(position);
                max_bounds = max_bounds.max(position);

                vertices.push(Vertex {
                    position: [position.x, position.y, position.z],
                    normal: [normal.x, normal.y, normal.z],
                    uv: [u, v],
                });
            }
        }

        // Generate indices (two triangles per quad)
        let mut indices = Vec::new();
        for i in 0..subdivisions {
            for j in 0..subdivisions {
                let row = subdivisions + 1;
                let v0 = i * row + j;
                let v1 = v0 + 1;
                let v2 = v0 + row;
                let v3 = v2 + 1;

                // First triangle
                indices.push(v0);
                indices.push(v2);
                indices.push(v1);

                // Second triangle
                indices.push(v1);
                indices.push(v2);
                indices.push(v3);
            }
        }

        // Create bounding box
        let bounding_box = ModelBoundingBox {
            min: min_bounds,
            max: max_bounds,
        };

        MeshAsset {
            label: label.to_string(),
            vertices,
            indices,
            bounding_box,
        }
    }
}

/// Compute tangent and bitangent vectors for a given normal.
/// This creates an orthonormal basis for the plane.
fn compute_tangent_bitangent(normal: Vec3) -> (Vec3, Vec3) {
    // Choose an arbitrary vector that's not parallel to normal
    let up = if normal.y.abs() > 0.9 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };

    let tangent = normal.cross(up).normalize();
    let bitangent = normal.cross(tangent).normalize();

    (tangent, bitangent)
}

