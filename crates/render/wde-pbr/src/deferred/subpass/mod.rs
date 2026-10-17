mod gbuffer_bindgroup;
mod gbuffer_pipeline;
mod gbuffer_subpass_pbr;
mod transparent_pbr;

pub use gbuffer_bindgroup::SsboTransformBinding;

use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::{
    deferred::subpass::transparent_pbr::{
        SubRenderPassTransparentPbr, TransparentPbrRenderPipeline, TransparentSceneBinding
    },
    deferred::subpass::{
        gbuffer_pipeline::GBufferRenderPipeline, gbuffer_subpass_pbr::SubRenderPassGbufferPbr
    },
    prelude::{RenderPassDeferredGBuffer, RenderPassTransparent}
};

pub(crate) struct PbrRenderPlugin;
impl Plugin for PbrRenderPlugin {
    fn build(&self, app: &mut App) {
        // Add the bind group
        app.add_plugins(RenderBindingRegisterPlugin::<SsboTransformBinding>::default())
            .add_plugins(RenderBindingRegisterPlugin::<TransparentSceneBinding>::default());

        // Add the pbr pipeline
        app.add_plugins(RenderPipelineRegisterPlugin::<GBufferRenderPipeline>::default())
            .add_plugins(RenderPipelineRegisterPlugin::<TransparentPbrRenderPipeline>::default());

        // Add the render graph nodes
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap()
            .add_sub_pass::<SubRenderPassGbufferPbr, RenderPassDeferredGBuffer>()
            .add_sub_pass::<SubRenderPassTransparentPbr, RenderPassTransparent>();
    }
}
