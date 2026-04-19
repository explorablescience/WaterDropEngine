use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::render::{
    dependencies::BuffersPlugin,
    extractor::TerrainExtractorPlugin,
    passes::TerrainPassesPlugin,
    renderer::TerrainRenderer,
    renderer_gpu::{TerrainRendererGPU, TerrainTileBgCompute, TerrainTileBgRender}
};

pub mod dependencies;
pub mod extractor;
mod passes;
pub mod renderer;
pub mod renderer_gpu;

pub struct TerrainRenderPlugin;
impl Plugin for TerrainRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BuffersPlugin)
            .add_plugins(TerrainPassesPlugin)
            .add_plugins(TerrainExtractorPlugin);

        // Add the terrain renderer resource and its systems
        // Note that using Update, the dirty tiles will be extracted to the GPU renderer resource one frame after they are marked as dirty. This is to ensure that the main world is not locked for too long while the GPU renderer resource is being updated.
        app.add_systems(Update, TerrainRenderer::extract_dirty);
        app.add_plugins((
            RenderBindingRegisterPlugin::<TerrainTileBgRender>::without_init(),
            RenderBindingRegisterPlugin::<TerrainTileBgCompute>::without_init()
        ));

        // Add the terrain renderer GPU resource and its systems
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
