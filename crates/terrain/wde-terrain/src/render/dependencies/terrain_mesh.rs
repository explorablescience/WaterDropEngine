use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::{
    manager::{CHUNK_RENDER_SUBDIVISIONS, CHUNK_SIZE},
    render::renderer::TERRAIN_LOD_COUNT
};

/// Flat chunk grids, one per level of detail, displaced by the heightmaps in the vertex shader.
#[derive(Resource, Default)]
pub struct TerrainRenderPassMesh {
    pub lod_meshes: Vec<Handle<Mesh>>
}
impl TerrainRenderPassMesh {
    // Creates the rendering meshes.
    pub fn init(assets_server: Res<AssetServer>, mut render_pass: ResMut<TerrainRenderPassMesh>) {
        render_pass.lod_meshes = (0..TERRAIN_LOD_COUNT)
            .map(|lod| {
                let subdivisions = CHUNK_RENDER_SUBDIVISIONS >> lod;
                let mut mesh = PlaneMesh::from(
                    &format!("terrain_tile_lod{lod}"),
                    subdivisions,
                    Vec3::Y,
                    false
                );
                // Cover the chunk, UVs in [0, 1] landing on the heightmap samples
                for vertex in &mut mesh.vertices {
                    vertex.position[0] *= CHUNK_SIZE;
                    vertex.position[2] *= CHUNK_SIZE;
                    vertex.uv[0] = vertex.position[0] / CHUNK_SIZE + 0.5;
                    vertex.uv[1] = vertex.position[2] / CHUNK_SIZE + 0.5;
                }
                assets_server.add(mesh)
            })
            .collect();
    }

    // Extracts the mesh handles to the render world.
    pub fn extract_terrain_mesh(
        mesh_cpu: ExtractWorld<Res<TerrainRenderPassMesh>>,
        mut mesh: ResMut<TerrainRenderPassMesh>
    ) {
        if mesh.lod_meshes.is_empty() && !mesh_cpu.lod_meshes.is_empty() {
            mesh.lod_meshes = mesh_cpu.lod_meshes.clone();
        }
    }
}
