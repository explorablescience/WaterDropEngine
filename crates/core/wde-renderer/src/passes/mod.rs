use bevy::prelude::*;
use depth_msaa::{DepthTextureMSAA};
use depth::{DepthTexture};

use crate::{assets::RenderAssetsPlugin, core::{Extract, Render, RenderApp, RenderSet}, passes::{depth_blit_pipeline::{DepthBlitRenderPipeline, DepthBlitRenderPipelineAsset, GpuDepthBlitRenderPipeline}, depth_blit_renderpass::DepthBlitRenderPass, depth_msaa::DepthMSAATextureLayout}, prelude::RenderGraph};

pub mod depth;
pub mod depth_msaa;
pub mod depth_blit_pipeline;
pub mod depth_blit_renderpass;
pub mod render_graph;

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

        // Add the creation of the mesh for the depth blit pass
        app
            .init_resource::<DepthBlitRenderPass>()
            .add_systems(Startup, DepthBlitRenderPass::init);
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<DepthBlitRenderPass>();

        // Add the extract system for the depth blit pass
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Extract, DepthBlitRenderPass::extract);

        // Add the depth blit render pass (always at binding 100)
        let mut render_graph = app.get_sub_app_mut(RenderApp).unwrap()
            .world_mut().get_resource_mut::<RenderGraph>().unwrap();
        render_graph.add_pass_old::<DepthBlitRenderPass>(100);
    }

    fn finish(&self, app: &mut App) {
        // Create the pipeline
        let pipeline = app.world_mut()
            .get_resource::<AssetServer>().unwrap().add(DepthBlitRenderPipelineAsset);
        app.get_sub_app_mut(RenderApp).unwrap().world_mut().spawn(DepthBlitRenderPipeline(pipeline));
    }
}
