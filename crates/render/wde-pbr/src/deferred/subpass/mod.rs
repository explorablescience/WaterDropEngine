mod gbuffer_pipeline;
mod gbuffer_subpass_pbr;

use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::{
    deferred::subpass::{
        gbuffer_pipeline::GBufferRenderPipeline, gbuffer_subpass_pbr::SubRenderPassGbufferPbr
    },
    prelude::RenderPassGBuffer
};

pub(crate) struct PbrRenderPlugin;
impl Plugin for PbrRenderPlugin {
    fn build(&self, app: &mut App) {
        // Add the pbr pipeline
        app.add_plugins(RenderPipelinePluginRegister::<GBufferRenderPipeline>::default());

        // Add the render graph nodes
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap()
            .add_sub_pass::<SubRenderPassGbufferPbr, RenderPassGBuffer>();
    }
}
