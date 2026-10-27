use bevy::prelude::*;
use wde_renderer::prelude::*;

pub(crate) mod pipeline;

use pipeline::StencilWriteRenderPipeline;

/// Pipeline writing the silhouette of outlined meshes to the stencil, drawn by the outline sub-pass.
pub(crate) struct StencilWritePlugin;
impl Plugin for StencilWritePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderPipelineRegisterPlugin::<StencilWriteRenderPipeline>::default());
    }
}
