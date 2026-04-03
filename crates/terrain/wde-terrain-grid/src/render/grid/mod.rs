use wde_renderer::prelude::*;

use bevy::prelude::*;

use crate::render::grid::{buffers::TerrainGridBufferPlugin, pipeline::{GpuTerrainGridRenderPipeline, TerrainGridRenderPipeline, TerrainGridRenderPipelineAsset}, renderpass::TerrainGridRenderPass};

mod pipeline;
mod renderpass;
mod buffers;

pub(crate) struct TerrainGridPassesPlugin;
impl Plugin for TerrainGridPassesPlugin {
    fn build(&self, app: &mut App) {
        // Add the render buffers
        app
            .get_sub_app_mut(RenderApp).unwrap()
            .add_plugins(TerrainGridBufferPlugin);

        // Add the pipelines
        app
            .init_asset::<TerrainGridRenderPipelineAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuTerrainGridRenderPipeline>::default());

        // Add the render pass
        app
            .init_resource::<TerrainGridRenderPass>()
            .add_systems(Startup, TerrainGridRenderPass::init);
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<TerrainGridRenderPass>()
            .add_systems(Extract, TerrainGridRenderPass::extract);

        // Add the render passes
        let mut render_graph = app.get_sub_app_mut(RenderApp).unwrap()
            .world_mut().get_resource_mut::<RenderGraph>().unwrap();
        render_graph.add_pass_old::<TerrainGridRenderPass>(120);
    }

    fn finish(&self, app: &mut App) {
        // Create the pipeline
        let pipeline = app.world_mut()
            .get_resource::<AssetServer>().unwrap().add(TerrainGridRenderPipelineAsset);
        app.get_sub_app_mut(RenderApp).unwrap().world_mut().spawn(TerrainGridRenderPipeline(pipeline));
    }
}

