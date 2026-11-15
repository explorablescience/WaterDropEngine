use meshopt::{SimplifyOptions, VertexDataAdapter};
use serde::{Deserialize, Serialize};
use wde_wgpu::utils::Vertex;

use super::Mesh;

/// Maximum number of levels of detail of a mesh, the full mesh included.
pub const MAX_MESH_LODS: usize = 4;

/// Simplified level of detail of a [`Mesh`], indexing the vertices of the full mesh.
#[derive(Clone, Debug)]
pub struct MeshLod {
    pub indices: Vec<u32>,
    /// Largest distance between this level and the full mesh surface, in mesh space.
    pub error: f32
}

/// Automatic generation of the levels of detail of a mesh, see [`Mesh::generate_lods`].
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MeshLodSettings {
    /// Levels to generate, the full mesh excluded (at most [`MAX_MESH_LODS`] - 1).
    pub levels: usize,
    /// Triangle count of each level, relative to the previous one.
    pub ratio: f32,
    /// Meshes with fewer triangles get no levels.
    pub min_triangles: usize,
    /// Largest error of a level, relative to the mesh size.
    pub max_error: f32,
    /// Generation stops at the first level removing less than this fraction of the triangles of
    /// the previous one.
    pub min_reduction: f32
}
impl Default for MeshLodSettings {
    fn default() -> Self {
        Self {
            levels: MAX_MESH_LODS - 1,
            ratio: 0.5,
            min_triangles: 64,
            max_error: 0.1,
            min_reduction: 0.2
        }
    }
}

impl Mesh {
    /// Replaces the [`Mesh::lods`] by simplified versions of the mesh, sharing its vertices.
    pub fn generate_lods(&mut self, settings: &MeshLodSettings) {
        self.lods.clear();
        if self.indices.len() / 3 < settings.min_triangles {
            return;
        }
        let Ok(adapter) = VertexDataAdapter::new(
            bytemuck::cast_slice(&self.vertices),
            std::mem::size_of::<Vertex>(),
            std::mem::offset_of!(Vertex, position)
        ) else {
            return;
        };
        let scale = meshopt::simplify_scale(&adapter);

        let mut previous_count = self.indices.len();
        let mut previous_error = 0.0f32;
        for _ in 0..settings.levels.min(MAX_MESH_LODS - 1) {
            let target = (previous_count as f32 * settings.ratio) as usize / 3 * 3;
            let mut error = 0.0;
            let indices = meshopt::simplify(
                &self.indices,
                &adapter,
                target,
                settings.max_error,
                SimplifyOptions::Prune,
                Some(&mut error)
            );
            if indices.is_empty()
                || indices.len() as f32 > previous_count as f32 * (1.0 - settings.min_reduction)
            {
                break;
            }
            previous_count = indices.len();
            // Simplifying further can give a smaller error: levels must get coarser
            previous_error = previous_error.max(error * scale);
            self.lods.push(MeshLod {
                indices: meshopt::optimize_vertex_cache(&indices, self.vertices.len()),
                error: previous_error
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::math::Vec3;

    use super::*;
    use crate::assets::MeshBbox;

    /// Wavy grid of `n` x `n` quads.
    fn grid(n: u32) -> Mesh {
        let mut vertices = Vec::new();
        for y in 0..=n {
            for x in 0..=n {
                let (u, v) = (x as f32 / n as f32, y as f32 / n as f32);
                vertices.push(Vertex {
                    position: [u, (u * 6.0).sin() * (v * 6.0).cos() * 0.1, v],
                    normal: [0.0, 1.0, 0.0],
                    uv: [u, v],
                    ..Default::default()
                });
            }
        }
        let mut indices = Vec::new();
        for y in 0..n {
            for x in 0..n {
                let i = y * (n + 1) + x;
                indices.extend([i, i + n + 1, i + 1, i + 1, i + n + 1, i + n + 2]);
            }
        }
        Mesh {
            label: "grid".to_string(),
            vertices,
            indices,
            lods: Vec::new(),
            bbox: MeshBbox {
                min: Vec3::ZERO,
                max: Vec3::ONE
            },
            use_ssbo: true
        }
    }

    #[test]
    fn levels_get_coarser() {
        let mut mesh = grid(32);
        mesh.generate_lods(&MeshLodSettings::default());
        assert_eq!(mesh.lods.len(), MAX_MESH_LODS - 1);

        let mut previous = (mesh.indices.len(), 0.0);
        for lod in &mesh.lods {
            assert!(lod.indices.len() < previous.0);
            assert!(lod.error >= previous.1);
            assert!(
                lod.indices
                    .iter()
                    .all(|&i| (i as usize) < mesh.vertices.len())
            );
            previous = (lod.indices.len(), lod.error);
        }
    }

    #[test]
    fn small_meshes_are_kept() {
        let mut mesh = grid(2);
        mesh.generate_lods(&MeshLodSettings::default());
        assert!(mesh.lods.is_empty());
    }
}
