use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::render::dependencies::{
    materials::TerrainMaterialsPlugin,
    terrain_buffer::{TerrainBuffer, TerrainBufferBinding, update_terrain_tiles_buffer},
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
        app.add_plugins((
            TerrainMaterialsPlugin,
            RenderDataRegisterPlugin::<TerrainBuffer>::default(),
            RenderBindingRegisterPlugin::<TerrainBufferBinding>::default()
        ));
        app.get_sub_app_mut(RenderApp).unwrap().add_systems(
            Render,
            update_terrain_tiles_buffer.in_set(RenderSet::Prepare)
        );

        // Init the render pass meshes
        app.init_resource::<TerrainRenderPassMesh>()
            .add_systems(Startup, TerrainRenderPassMesh::init);
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<TerrainRenderPassMesh>();
    }
}
