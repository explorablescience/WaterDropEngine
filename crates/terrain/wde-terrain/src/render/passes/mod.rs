use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

use bevy::prelude::*;

use crate::render::passes::{
    pipeline::TerrainRenderPipeline, subpass_terrain_ground::SubRenderPassTerrainGround
};

mod pipeline;
mod subpass_terrain_ground;

pub(crate) struct TerrainPassesPlugin;
impl Plugin for TerrainPassesPlugin {
    fn build(&self, app: &mut App) {
        // Add the pipelines
        app.add_plugins(RenderPipelineRegisterPlugin::<TerrainRenderPipeline>::default());

        // Add the extract system for the render pass
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .add_systems(Extract, SubRenderPassTerrainGround::extract);
    }

    fn finish(&self, app: &mut App) {
        // Add the terrain render passes
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap()
            // .add_pass::<RenderPassTerrain>()
            .add_sub_pass::<SubRenderPassTerrainGround, RenderPassDeferredGBuffer>();
    }
}
