use bevy::prelude::*;
use depth_texture_msaa::{DepthTextureMSAA};
use depth_texture::{DepthTexture};

use crate::{assets::RenderAssetsPlugin, core::{Extract, Render, RenderApp, RenderSet}, passes::depth::{depth_blit_pipeline::{DepthBlitRenderPipeline, DepthBlitRenderPipelineAsset, GpuDepthBlitRenderPipeline}, depth_blit_renderpass::RenderPassDepthBlit, depth_blit_subpass::SubRenderPassDepthBlit, depth_texture_msaa::DepthMSAATextureLayout}, prelude::RenderGraph};

pub mod depth_texture;
pub mod depth_texture_msaa;

mod depth_blit_pipeline;
mod depth_blit_renderpass;
mod depth_blit_subpass;

pub(crate) struct RendererPlugin;
impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        // Add the depth MSAA texture to the app
        app
            .add_systems(Startup, DepthTextureMSAA::init)
            .add_systems(Update, DepthTextureMSAA::resize);
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Extract, DepthTextureMSAA::extract)
            .init_resource::<DepthMSAATextureLayout>()
            .add_systems(Render, DepthMSAATextureLayout::build_bind_group.in_set(RenderSet::BindGroups));

        // Add the depth texture to the app
        app
            .add_systems(Startup, DepthTexture::create_texture)
            .add_systems(Update, DepthTexture::resize_texture);
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Extract, DepthTexture::extract_texture);

        // Add the depth blit pipeline
        app
            .init_asset::<DepthBlitRenderPipelineAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuDepthBlitRenderPipeline>::default());

        // Add the depth blit render pass (always at binding 100)
        app.get_sub_app_mut(RenderApp).unwrap()
            .world_mut().get_resource_mut::<RenderGraph>().unwrap()
            .add_pass::<RenderPassDepthBlit>()
            .add_sub_pass::<SubRenderPassDepthBlit, RenderPassDepthBlit>();
    }

    fn finish(&self, app: &mut App) {
        // Create the pipeline
        let pipeline = app.world_mut()
            .get_resource::<AssetServer>().unwrap().add(DepthBlitRenderPipelineAsset);
        app.get_sub_app_mut(RenderApp).unwrap().world_mut().spawn(DepthBlitRenderPipeline(pipeline));
    }
}
