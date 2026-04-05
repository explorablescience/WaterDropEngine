use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::render::dependencies::{
    materials::{TerrainMaterialArrays, TerrainMaterialsPlugin},
    terrain_buffer::TerrainBufferPlugin,
    terrain_mesh::TerrainRenderPassMesh
};

pub mod materials;
pub mod terrain_buffer;
pub mod terrain_mesh;

pub struct BuffersPlugin;
impl Plugin for BuffersPlugin {
    fn build(&self, app: &mut App) {
        // Add the terrain mesh buffer
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .add_systems(Extract, TerrainRenderPassMesh::extract_terrain_mesh);

        // Init the terrain material arrays
        app.add_plugins(TerrainMaterialsPlugin);
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<TerrainMaterialArrays>()
            .add_plugins(TerrainBufferPlugin);

        // Init the render pass meshes
        app.init_resource::<TerrainRenderPassMesh>()
            .add_systems(Startup, TerrainRenderPassMesh::init);
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<TerrainRenderPassMesh>();
    }
}
