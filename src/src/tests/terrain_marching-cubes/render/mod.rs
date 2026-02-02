use bevy::prelude::*;
use render_core::MCRenderPass;
use render_pipeline::{GpuMCRenderPipeline, MCRenderPipeline, MCRenderPipelineAsset};
use wde_render::{assets::RenderAssetsPlugin, core::RenderApp, passes::render_graph::RenderGraph};

mod render_core;
mod render_pipeline;

pub struct MCRenderPlugin;
impl Plugin for MCRenderPlugin {
    fn build(&self, app: &mut App) {
        // Register the render pipeline asset
        app
            .init_asset::<MCRenderPipelineAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuMCRenderPipeline>::default());

        // Render pass
        let mut render_graph = app.get_sub_app_mut(RenderApp).unwrap()
            .world_mut().get_resource_mut::<RenderGraph>().unwrap();
        render_graph.add_pass::<MCRenderPass>(100);
    }

    fn finish(&self, app: &mut App) {
        // Create the render pipeline instance
        let pipeline = app.world_mut()
            .get_resource::<AssetServer>().unwrap().add(MCRenderPipelineAsset);
        app.get_sub_app_mut(RenderApp).unwrap().world_mut().spawn(MCRenderPipeline(pipeline));
    }
}
