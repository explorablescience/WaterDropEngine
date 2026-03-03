use wde_renderer::prelude::*;

use bevy::prelude::*;

use crate::render::passes::{tiles_extractor::GpuTerrainTiles, pipeline::{GpuTerrainRenderPipeline, TerrainRenderPipeline, TerrainRenderPipelineAsset}, renderpass::TerrainRenderPass};

pub mod tiles_extractor;
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
            .init_resource::<TerrainRenderPass>()
            .init_resource::<GpuTerrainTiles>()
            .add_systems(Render, GpuTerrainTiles::prepare_tiles.in_set(RenderSet::BindGroups));
        if cfg!(debug_assertions) {
            app.get_sub_app_mut(RenderApp).unwrap()
                .add_systems(Render, GpuTerrainTiles::check_dirty_tiles.in_set(RenderSet::Prepare));
        }

        // Add the terrain render passes
        let mut render_graph = app.get_sub_app_mut(RenderApp).unwrap()
            .world_mut().get_resource_mut::<RenderGraph>().unwrap();
        render_graph.add_pass::<TerrainRenderPass>(110);
    }

    fn finish(&self, app: &mut App) {
        // Create the pipeline
        let pipeline = app.world_mut()
            .get_resource::<AssetServer>().unwrap().add(TerrainRenderPipelineAsset);
        app.get_sub_app_mut(RenderApp).unwrap().world_mut().spawn(TerrainRenderPipeline(pipeline));
    }
}

