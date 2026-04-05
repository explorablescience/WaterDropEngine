use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

use bevy::prelude::*;

use crate::render::grid::{
    buffers::TerrainGridBufferPlugin,
    terrain_grid_pipeline::{
        GpuTerrainGridRenderPipeline, TerrainGridRenderPipeline, TerrainGridRenderPipelineAsset
    },
    terrain_grid_subpass::RenderSubPassTerrainGrid
};

mod buffers;
mod terrain_grid_pipeline;
mod terrain_grid_subpass;

pub(crate) struct TerrainGridPassesPlugin;
impl Plugin for TerrainGridPassesPlugin {
    fn build(&self, app: &mut App) {
        // Add the render buffers
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .add_plugins(TerrainGridBufferPlugin);

        // Add the pipelines
        app.init_asset::<TerrainGridRenderPipelineAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuTerrainGridRenderPipeline>::default());

        // Add the mesh for the terrain grid
        app.init_resource::<RenderSubPassTerrainGrid>()
            .add_systems(Startup, RenderSubPassTerrainGrid::init);
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<RenderSubPassTerrainGrid>()
            .add_systems(Extract, RenderSubPassTerrainGrid::extract);
    }

    fn finish(&self, app: &mut App) {
        // Create the pipeline
        let pipeline = app
            .world_mut()
            .get_resource::<AssetServer>()
            .unwrap()
            .add(TerrainGridRenderPipelineAsset);
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .world_mut()
            .spawn(TerrainGridRenderPipeline(pipeline));

        // Add the render passes
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap()
            .add_sub_pass::<RenderSubPassTerrainGrid, RenderPassTransparent>();
    }
}
