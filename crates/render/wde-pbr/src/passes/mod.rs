use wde_renderer::prelude::*;

use bevy::prelude::*;

mod pipeline_gbuffer;
mod renderpass_gbuffer;
mod pipeline_lighting;
mod renderpass_lighting;
mod subpass_gbuffer_pbr;
mod subpass_lighting_pbr;

use renderpass_gbuffer::*;
use renderpass_lighting::*;

use crate::passes::{pipeline_gbuffer::{GpuPbrGBufferRenderPipeline, PbrGBufferRenderPipeline, PbrGBufferRenderPipelineAsset}, pipeline_lighting::{GpuPbrLightingRenderPipeline, PbrLightingRenderPipeline, PbrLightingRenderPipelineAsset}, subpass_gbuffer_pbr::SubRenderPassGbufferPbr, subpass_lighting_pbr::SubRenderPassLightingPbr};

pub(crate) struct PbrFeaturesPlugin;
impl Plugin for PbrFeaturesPlugin {
    fn build(&self, app: &mut App) {
        // Add the pbr pipelines
        app
            .init_asset::<PbrGBufferRenderPipelineAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuPbrGBufferRenderPipeline>::default())
            .init_asset::<PbrLightingRenderPipelineAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuPbrLightingRenderPipeline>::default());

        // Add the systems for the render passes
        app
            .init_resource::<PbrLightingRenderPassMesh>()
            .add_systems(Startup, PbrLightingRenderPassMesh::init);
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<PbrLightingRenderPassMesh>();
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Extract, PbrLightingRenderPassMesh::extract);

        // Add the render graph nodes
        app.get_sub_app_mut(RenderApp).unwrap().world_mut()
            .get_resource_mut::<RenderGraph>().unwrap()
            .add_pass::<RenderPassGBuffer>()
            .add_sub_pass::<SubRenderPassGbufferPbr, RenderPassGBuffer>()
            .add_pass::<PbrLightingRenderPass>()
            .add_sub_pass::<SubRenderPassLightingPbr, PbrLightingRenderPass>();
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
    }
}

