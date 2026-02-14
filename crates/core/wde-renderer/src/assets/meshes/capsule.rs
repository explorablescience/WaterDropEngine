use bevy::math::Vec3;
use wde_wgpu::vertex::Vertex;

use crate::assets::{MeshAsset, ModelBoundingBox};

pub struct CapsuleMeshConfig {
    pub radius: f32,
    pub height: f32,
    pub segments: u32,
    pub rings: u32,
}
impl Default for CapsuleMeshConfig {
    fn default() -> Self {
        Self {
            radius: 0.5,
            height: 2.0,
            segments: 16,
            rings: 8
        }
    }
}

pub struct CapsuleMesh;
impl CapsuleMesh {
    /// Create a new capsule mesh.
    /// The capsule is centered at the origin and extends along the Y axis.
    /// 
    /// # Arguments
    /// 
    /// - `label`: A label for the mesh asset.
    /// - `config`: Configuration parameters for the capsule mesh.
    /// 
    /// # Returns
    /// 
    /// The capsule mesh.
    pub fn from(label: &str, config: CapsuleMeshConfig) -> MeshAsset {
        let (radius, height, segments, rings) = (config.radius, config.height, config.segments, config.rings);
        
        let segments = segments.max(3);
        let rings = rings.max(1);
        
        // Calculate cylinder height (total height minus the two hemisphere radii)
        let cylinder_height = (height - 2.0 * radius).max(0.0);
        let half_cylinder_height = cylinder_height / 2.0;
        
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        // Generate top hemisphere
        for ring in 0..=rings {
            let phi = std::f32::consts::PI * 0.5 * (ring as f32) / (rings as f32);
            let y = radius * phi.cos() + half_cylinder_height;
            let ring_radius = radius * phi.sin();
            
            for segment in 0..=segments {
                let theta = 2.0 * std::f32::consts::PI * (segment as f32) / (segments as f32);
                let x = ring_radius * theta.cos();
                let z = ring_radius * theta.sin();
                
                // Normal for sphere is just the normalized position vector
                let normal_x = x;
                let normal_y = radius * phi.cos();
                let normal_z = z;
                let normal_length = (normal_x * normal_x + normal_y * normal_y + normal_z * normal_z).sqrt();
                
                // UV coordinates
                let u = (segment as f32) / (segments as f32);
                let v = 1.0 - (ring as f32) / (rings as f32) * 0.25; // Top hemisphere uses top 25% of texture
                
                vertices.push(Vertex {
                    position: [x, y, z],
                    normal: [normal_x / normal_length, normal_y / normal_length, normal_z / normal_length],
                    uv: [u, v],
                    tangent: [0.0, 0.0, 0.0, 0.0],
                });
            }
        }
        
        // Generate cylinder body
        for ring in 0..=1 {
            let y = if ring == 0 { half_cylinder_height } else { -half_cylinder_height };
            
            for segment in 0..=segments {
                let theta = 2.0 * std::f32::consts::PI * (segment as f32) / (segments as f32);
                let x = radius * theta.cos();
                let z = radius * theta.sin();
                
                // Normal points outward horizontally
                let normal_x = theta.cos();
                let normal_z = theta.sin();
                
                // UV coordinates
                let u = (segment as f32) / (segments as f32);
                let v = 0.75 - (ring as f32) * 0.5; // Cylinder uses middle 50% of texture
                
                vertices.push(Vertex {
                    position: [x, y, z],
                    normal: [normal_x, 0.0, normal_z],
                    uv: [u, v],
                    tangent: [0.0, 0.0, 0.0, 0.0],
                });
            }
        }
        
        // Generate bottom hemisphere
        for ring in 0..=rings {
            let phi = std::f32::consts::PI * 0.5 * (ring as f32) / (rings as f32);
            let y = -radius * phi.cos() - half_cylinder_height;
            let ring_radius = radius * phi.sin();
            
            for segment in 0..=segments {
                let theta = 2.0 * std::f32::consts::PI * (segment as f32) / (segments as f32);
                let x = ring_radius * theta.cos();
                let z = ring_radius * theta.sin();
                
                // Normal for sphere is just the normalized position vector
                let normal_x = x;
                let normal_y = -radius * phi.cos();
                let normal_z = z;
                let normal_length = (normal_x * normal_x + normal_y * normal_y + normal_z * normal_z).sqrt();
                
                // UV coordinates
                let u = (segment as f32) / (segments as f32);
                let v = 0.25 - (ring as f32) / (rings as f32) * 0.25; // Bottom hemisphere uses bottom 25% of texture
                
                vertices.push(Vertex {
                    position: [x, y, z],
                    normal: [normal_x / normal_length, normal_y / normal_length, normal_z / normal_length],
                    uv: [u, v],
                    tangent: [0.0, 0.0, 0.0, 0.0],
                });
            }
        }
        
        // Generate indices for top hemisphere
        let mut vertex_offset = 0u32;
        for ring in 0..rings {
            for segment in 0..segments {
                let current = vertex_offset + ring * (segments + 1) + segment;
                let next = current + segments + 1;
                
                indices.push(current);
                indices.push(next);
                indices.push(current + 1);
                
                indices.push(current + 1);
                indices.push(next);
                indices.push(next + 1);
            }
        }
        
        // Generate indices for cylinder
        vertex_offset += (rings + 1) * (segments + 1);
        for segment in 0..segments {
            let current = vertex_offset + segment;
            let next = current + segments + 1;
            
            indices.push(current);
            indices.push(next);
            indices.push(current + 1);
            
            indices.push(current + 1);
            indices.push(next);
            indices.push(next + 1);
        }
        
        // Generate indices for bottom hemisphere
        vertex_offset += 2 * (segments + 1);
        for ring in 0..rings {
            for segment in 0..segments {
                let current = vertex_offset + ring * (segments + 1) + segment;
                let next = current + segments + 1;
                
                indices.push(current);
                indices.push(next);
                indices.push(current + 1);
                
                indices.push(current + 1);
                indices.push(next);
                indices.push(next + 1);
            }
        }
        
        // Create bounding box
        let half_height = height / 2.0;
        let bounding_box = ModelBoundingBox {
            min: Vec3::new(-radius, -half_height, -radius),
            max: Vec3::new(radius, half_height, radius),
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
