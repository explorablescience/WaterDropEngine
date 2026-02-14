use wde_renderer::prelude::*;

use bevy::prelude::*;

use crate::render::passes::{pipeline::{GpuTerrainRenderPipeline, TerrainRenderPipeline, TerrainRenderPipelineAsset}, renderpass::{TerrainRenderPass, TerrainRenderPassMesh}};

mod pipeline;
mod renderpass;

pub(crate) struct TerrainRenderFeaturesPlugin;
impl Plugin for TerrainRenderFeaturesPlugin {
    fn build(&self, app: &mut App) {
        // Add the pipelines
        app
            .init_asset::<TerrainRenderPipelineAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuTerrainRenderPipeline>::default());

        // Init the render pass meshes
        app
            .init_resource::<TerrainRenderPassMesh>()
            .add_systems(Startup, TerrainRenderPassMesh::init);
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<TerrainRenderPassMesh>();

        // Add the terrain render passes
        let mut render_graph = app.get_sub_app_mut(RenderApp).unwrap()
            .world_mut().get_resource_mut::<RenderGraph>().unwrap();
        render_graph.add_pass::<TerrainRenderPass>(10);
    }

    fn finish(&self, app: &mut App) {
        // Create the pipeline
        let pipeline = app.world_mut()
            .get_resource::<AssetServer>().unwrap().add(TerrainRenderPipelineAsset);
        app.get_sub_app_mut(RenderApp).unwrap().world_mut().spawn(TerrainRenderPipeline(pipeline));
    }
}

