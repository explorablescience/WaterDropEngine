use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

use bevy::prelude::*;

use crate::render::passes::{
    pipeline::{GpuTerrainRenderPipeline, TerrainRenderPipeline, TerrainRenderPipelineAsset},
    subpass_terrain_ground::SubRenderPassTerrainGround
};

mod pipeline;
mod subpass_terrain_ground;

pub(crate) struct TerrainPassesPlugin;
impl Plugin for TerrainPassesPlugin {
    fn build(&self, app: &mut App) {
        // Add the pipelines
        app.init_asset::<TerrainRenderPipelineAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuTerrainRenderPipeline>::default());

        // Add the extract system for the render pass
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .add_systems(Extract, SubRenderPassTerrainGround::extract);
    }

    fn finish(&self, app: &mut App) {
        // Create the pipeline
        let pipeline = app
            .world_mut()
            .get_resource::<AssetServer>()
            .unwrap()
            .add(TerrainRenderPipelineAsset);
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .world_mut()
            .spawn(TerrainRenderPipeline(pipeline));

        // Add the terrain render passes
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap()
            // .add_pass::<RenderPassTerrain>()
            .add_sub_pass::<SubRenderPassTerrainGround, RenderPassOpaqueGBuffer>();
    }
}
