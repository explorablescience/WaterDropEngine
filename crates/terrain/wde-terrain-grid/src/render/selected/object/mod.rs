use wde_renderer::prelude::*;

use bevy::prelude::*;

use crate::render::selected::object::{pipeline::{GpuSelectedObjectRenderPipeline, SelectedObjectRenderPipeline, SelectedObjectRenderPipelineAsset}, renderpass::SelectedObjectRenderPass};

mod pipeline;
mod renderpass;

pub(crate) struct SelectedObjectPassesPlugin;
impl Plugin for SelectedObjectPassesPlugin {
    fn build(&self, app: &mut App) {
        // // Add the pipelines
        // app
        //     .init_asset::<SelectedObjectRenderPipelineAsset>()
        //     .add_plugins(RenderAssetsPlugin::<GpuSelectedObjectRenderPipeline>::default());

        // // Add the render pass
        // app.get_sub_app_mut(RenderApp).unwrap()
        //     .init_resource::<SelectedObjectRenderPass>();

        // // Add the render pass
        // let mut render_graph = app.get_sub_app_mut(RenderApp).unwrap()
        //     .world_mut().get_resource_mut::<RenderGraph>().unwrap();
        // render_graph.add_pass::<SelectedObjectRenderPass>(120);
    }

    fn finish(&self, app: &mut App) {
        // // Create the pipeline
        // let pipeline = app.world_mut()
        //     .get_resource::<AssetServer>().unwrap().add(SelectedObjectRenderPipelineAsset);
        // app.get_sub_app_mut(RenderApp).unwrap().world_mut().spawn(SelectedObjectRenderPipeline(pipeline));
    }
}

