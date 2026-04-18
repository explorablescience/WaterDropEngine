pub(crate) mod ghost_material;
mod ghost_pipeline;
mod ghost_subpass;

use bevy::prelude::*;
use wde_renderer::prelude::*;
use wde_pbr::prelude::*;

use crate::render::ghost::{ghost_material::GhostMaterial, ghost_pipeline::GhostRenderPipeline, ghost_subpass::{SubRenderPassGhost, extract_entities}};

pub(crate) struct GhostPlugin;
impl Plugin for GhostPlugin {
    fn build(&self, app: &mut App) {
        // Register the material
        app.init_asset::<GhostMaterial>()
            .add_plugins(RenderBindingRegisterPlugin::<GhostMaterial>::default());
        
        // Add the pbr pipeline
        app.add_plugins(RenderPipelineRegisterPlugin::<GhostRenderPipeline>::default());
    }

    fn finish(&self, app: &mut App) {
        // Add the render graph nodes
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .add_systems(Extract, extract_entities)
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap()
            .add_sub_pass::<SubRenderPassGhost, RenderPassTransparent>();
    }
}
