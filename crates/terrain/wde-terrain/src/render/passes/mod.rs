use wde_renderer::prelude::*;

use bevy::prelude::*;

use crate::render::passes::{pipeline::{GpuTerrainRenderPipeline, TerrainRenderPipeline, TerrainRenderPipelineAsset}, renderpass::TerrainRenderPass};

mod pipeline;
mod renderpass;

pub(crate) struct TerrainPassesPlugin;
impl Plugin for TerrainPassesPlugin {
    fn build(&self, app: &mut App) {
        // Add the pipelines
        app
            .init_asset::<TerrainRenderPipelineAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuTerrainRenderPipeline>::default());

        // Add the render pass
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<TerrainRenderPass>();

        // Add the extract system for the render pass
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Extract, TerrainRenderPass::extract);

        // Add the terrain render passes
        let mut render_graph = app.get_sub_app_mut(RenderApp).unwrap()
            .world_mut().get_resource_mut::<RenderGraph>().unwrap();
        render_graph.add_pass_old::<TerrainRenderPass>(110);
    }

    fn finish(&self, app: &mut App) {
        // Create the pipeline
        let pipeline = app.world_mut()
            .get_resource::<AssetServer>().unwrap().add(TerrainRenderPipelineAsset);
        app.get_sub_app_mut(RenderApp).unwrap().world_mut().spawn(TerrainRenderPipeline(pipeline));
    }
}

