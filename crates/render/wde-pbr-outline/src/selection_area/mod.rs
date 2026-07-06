use bevy::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

pub(crate) mod extract;
mod material;
mod pipeline;
mod subpass;

pub use material::SelectionAreaMaterial;

use pipeline::SelectionAreaRenderPipeline;
use subpass::SubRenderPassSelectionArea;

/// Registers the render pipeline and forward transparent subpass used to draw the
/// RTS-style ground selection decal (see [`SelectionAreaMaterial`]).
pub struct DrawSelectionAreaPlugin;
impl Plugin for DrawSelectionAreaPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderPipelineRegisterPlugin::<SelectionAreaRenderPipeline>::default());
    }

    fn finish(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap()
            .add_sub_pass::<SubRenderPassSelectionArea, RenderPassTransparent>();
    }
}
