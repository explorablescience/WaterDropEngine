use bevy::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

use crate::stencil_write::{
    pipeline::StencilWriteRenderPipeline, subpass::SubRenderPassStencilWrite
};

mod pipeline;
mod subpass;

pub(crate) struct StencilWritePlugin;
impl Plugin for StencilWritePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderPipelineRegisterPlugin::<StencilWriteRenderPipeline>::default());
    }

    fn finish(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap()
            .add_sub_pass::<SubRenderPassStencilWrite, RenderPassDeferredGBuffer>();
    }
}
