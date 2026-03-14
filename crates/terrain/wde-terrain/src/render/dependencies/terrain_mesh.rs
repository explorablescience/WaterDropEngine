use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::manager::{CHUNK_RENDER_SUBDIVISIONS, CHUNK_SIZE};

#[derive(Resource, Default)]
pub(crate) struct TerrainRenderPassMesh {
    pub deferred_mesh: Option<Handle<MeshAsset>>,
}
impl TerrainRenderPassMesh {
    // Creates the rendering mesh.
    pub fn init(assets_server: Res<AssetServer>, mut render_pass: ResMut<TerrainRenderPassMesh>) {
        let mut mesh = PlaneMesh::from("terrain_tile", CHUNK_RENDER_SUBDIVISIONS, Vec3::Y);

        // Scale the plane to cover the entire terrain tile size
        for vertex in &mut mesh.vertices {
            vertex.position[0] *= CHUNK_SIZE;
            vertex.position[2] *= CHUNK_SIZE;
        }

        // Fix the UVs to cover the entire texture
        for vertex in &mut mesh.vertices {
            vertex.uv[0] = vertex.position[0] / CHUNK_SIZE + 0.5; // Map from [-TILE_SIZE/2, TILE_SIZE/2] to [0, 1]
            vertex.uv[1] = vertex.position[2] / CHUNK_SIZE + 0.5; // Map from [-TILE_SIZE/2, TILE_SIZE/2] to [0, 1]
        }

        render_pass.deferred_mesh = Some(assets_server.add(mesh));
    }

    // Extracts the mesh handle to the render world.
    pub fn extract_terrain_mesh(
        mesh_cpu: ExtractWorld<Res<TerrainRenderPassMesh>>,
        mut mesh: ResMut<TerrainRenderPassMesh>,
    ) {
        // Check if the mesh is already extracted
        if mesh.deferred_mesh.is_some() {
            return;
        }
        
        // Extract the mesh handle to the render world
        if let Some(ref mesh_cpu) = mesh_cpu.deferred_mesh {
            mesh.deferred_mesh = Some(mesh_cpu.clone());
        }
    }
}
