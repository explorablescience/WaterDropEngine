use wde_renderer::prelude::*;

use bevy::prelude::*;

mod pbr_pipeline_gbuffer;
mod pbr_renderpass_gbuffer;
mod pbr_pipeline_lighting;
mod pbr_renderpass_lighting;
mod pbr_ssbo;
mod pbr_textures;

pub use pbr_renderpass_gbuffer::*;
pub use pbr_renderpass_lighting::*;

use crate::passes::{pbr_pipeline_gbuffer::{GpuPbrGBufferRenderPipeline, PbrGBufferRenderPipeline, PbrGBufferRenderPipelineAsset}, pbr_pipeline_lighting::{GpuPbrLightingRenderPipeline, PbrLightingRenderPipeline, PbrLightingRenderPipelineAsset}, pbr_ssbo::PbrSsboPlugin, pbr_textures::{PbrDeferredTextures, PbrDeferredTexturesLayout}};

pub(crate) struct PbrFeaturesPlugin;
impl Plugin for PbrFeaturesPlugin {
    fn build(&self, app: &mut App) {
        // Add the pbr ssbo
        app
            .add_plugins(PbrSsboPlugin);

        // Add the pbr defered textures
        app
            .add_systems(Startup, PbrDeferredTextures::create_textures)
            .add_systems(Update, PbrDeferredTextures::resize_textures);
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<PbrDeferredTexturesLayout>()
            .add_systems(Extract, PbrDeferredTextures::extract_textures)
            .add_systems(Render, PbrDeferredTexturesLayout::build_bind_group.in_set(RenderSet::BindGroups));

        // Add the pbr pipelines
        app
            .init_asset::<PbrGBufferRenderPipelineAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuPbrGBufferRenderPipeline>::default())
            .init_asset::<PbrLightingRenderPipelineAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuPbrLightingRenderPipeline>::default());

        // Init the render graph
        app
            .init_resource::<PbrLightingRenderPassMesh>()
            .add_systems(Startup, PbrLightingRenderPassMesh::init);
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<PbrLightingRenderPassMesh>();
    }

    fn finish(&self, app: &mut App) {
        // Create the render pass
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<PbrGBufferRenderPass>();

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

