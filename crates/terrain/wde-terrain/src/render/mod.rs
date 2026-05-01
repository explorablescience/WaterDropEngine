use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::render::{
    dependencies::BuffersPlugin,
    extractor::TerrainExtractorPlugin,
    passes::TerrainPassesPlugin,
    renderer::TerrainRenderer,
    renderer_gpu::{TerrainChunkArrayBg, TerrainComputeArrayBg, TerrainRendererGPU}
};

pub mod dependencies;
pub mod extractor;
mod passes;
pub mod renderer;
pub mod renderer_gpu;

pub struct TerrainRenderPlugin;
impl Plugin for TerrainRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((BuffersPlugin, TerrainPassesPlugin, TerrainExtractorPlugin));

        // Extract dirty tiles from Terrain → TerrainRenderer each frame.
        app.add_systems(Update, TerrainRenderer::extract_dirty);

        // Register the single-instance bind group types (created on demand).
        app.add_plugins((
            RenderBindingRegisterPlugin::<TerrainChunkArrayBg>::without_init(),
            RenderBindingRegisterPlugin::<TerrainComputeArrayBg>::without_init()
        ));

        // GPU-side systems: upload dirty tiles, then create bind groups if needed.
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<TerrainRendererGPU>()
            .add_systems(
                Render,
                (
                    TerrainRendererGPU::upload_dirty,
                    TerrainRendererGPU::prepare_bind_groups
                )
                    .chain()
                    .in_set(RenderSet::Prepare)
            );
    }
}
