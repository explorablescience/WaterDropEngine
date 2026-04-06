//! The render pass for resolving the multisampled [`crate::logic::render_texture::RenderTexture`], which is the color attachment of the deferred lighting pass, to the swapchain texture, which is then presented on the screen.
use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_renderer::prelude::*;

use crate::prelude::RenderTextureBindGroup;

/// The render pass for resolving the multisampled [`crate::logic::render_texture::RenderTexture`], which is the color attachment of the deferred lighting pass, to the swapchain texture, which is then presented on the screen.
///
/// Note:
///  - This render pass has a render index of 100. It should be executed after all the rendering passes that render to the multisampled render texture.
pub(crate) struct RenderPassResolve;
impl RenderPass for RenderPassResolve {
    type Params = ();

    fn describe(_: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc::default()
    }
    fn id() -> RenderPassId {
        100
    }
    fn label() -> &'static str {
        "resolve"
    }
}

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct ResolveRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for ResolveRenderPipeline {
    type SourceAsset = RenderPipelineAsset<ResolveRenderPipeline>;
    type Params = (SRes<AssetServer>, SResMut<PipelineManager>);

    fn prepare(
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager): &mut bevy::ecs::system::SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(ResolveRenderPipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "resolve",
                    vert: Some(assets_server.load("core/render/resolve/vert.wgsl")),
                    frag: Some(assets_server.load("core/render/resolve/frag.wgsl")),
                    bind_group_layouts: vec![Some(RenderTextureBindGroup::layout())],
                    ..Default::default()
                },
                asset
            )?
        ))
    }
}

pub(crate) struct SubRenderPassResolve;
impl RenderSubPass for SubRenderPassResolve {
    type Params = (
        SRes<RenderAssets<ResolveRenderPipeline>>,
        SRes<PostProcessingMesh>,
        SRes<RenderTextureBindGroup>
    );

    fn describe(
        (pipeline, mesh, pbr_texture): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(Some(pipeline.iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::Mesh(mesh.0.as_ref().map(|h| h.id())),
            SubPassCommand::BindGroup(0, pbr_texture.bind_group.clone()),
            SubPassCommand::DrawBatches(vec![DrawCommandsBatch {
                index_range: 0..6,
                ..Default::default()
            }]),
        ])
    }

    fn label() -> &'static str {
        "resolve"
    }
}
