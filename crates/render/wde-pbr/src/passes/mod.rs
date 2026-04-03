use wde_renderer::prelude::*;

use bevy::prelude::*;

mod core;
mod subpass;

pub use core::opaque_gbuffer_renderpass::RenderPassOpaqueGBuffer;
pub use core::opaque_lighting_renderpass::RenderPassOpaqueLighting;

use core::{resolve::*};

use crate::passes::{core::{transparent_renderpass::TransparentRenderPass}, subpass::{
    gbuffer_pipeline::*, gbuffer_subpass_pbr::SubRenderPassGbufferPbr, lighting_pipeline::*, lighting_subpass_pbr::SubRenderPassLightingPbr
}};

pub(crate) struct PbrFeaturesPlugin;
impl Plugin for PbrFeaturesPlugin {
    fn build(&self, app: &mut App) {
        // Add the pbr pipelines
        app
            .init_asset::<PbrGBufferRenderPipelineAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuPbrGBufferRenderPipeline>::default())
            .init_asset::<PbrLightingRenderPipelineAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuPbrLightingRenderPipeline>::default());

        // Add the depth blit pipeline
        app
            .init_asset::<ResolveRenderPipelineAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuResolveRenderPipeline>::default());

        // Add the render graph nodes
        app.get_sub_app_mut(RenderApp).unwrap().world_mut()
            .get_resource_mut::<RenderGraph>().unwrap()
            .add_pass::<RenderPassOpaqueGBuffer>()
            .add_sub_pass::<SubRenderPassGbufferPbr, RenderPassOpaqueGBuffer>()
            .add_pass::<RenderPassOpaqueLighting>()
            .add_sub_pass::<SubRenderPassLightingPbr, RenderPassOpaqueLighting>()
            .add_pass::<TransparentRenderPass>()
            .add_pass::<RenderPassResolve>()
            .add_sub_pass::<SubRenderPassResolve, RenderPassResolve>();
    }

    fn finish(&self, app: &mut App) {
        // Create the gbuffer pipeline
        let pipeline = app.world_mut()
            .get_resource::<AssetServer>().unwrap().add(PbrGBufferRenderPipelineAsset);
        app.get_sub_app_mut(RenderApp).unwrap().world_mut().spawn(PbrGBufferRenderPipeline(pipeline));

        // Create the lighting pipeline
        let pipeline = app.world_mut()
            .get_resource::<AssetServer>().unwrap().add(PbrLightingRenderPipelineAsset);
        app.get_sub_app_mut(RenderApp).unwrap().world_mut().spawn(PbrLightingRenderPipeline(pipeline));

        // Create the depth blit pipeline
        let pipeline = app.world_mut()
            .get_resource::<AssetServer>().unwrap().add(ResolveRenderPipelineAsset);
        app.get_sub_app_mut(RenderApp).unwrap().world_mut().spawn(ResolveRenderPipeline(pipeline));
    }
}

