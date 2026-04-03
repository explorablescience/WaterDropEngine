use wde_renderer::{passes::depth::depth_texture_msaa::DepthMSAATextureBindGroup, prelude::*};
use bevy::{ecs::system::{SystemParamItem, lifetimeless::{SRes, SResMut}}, prelude::*};

use crate::logic::render_texture::PbrRenderTextureBindGroup;

pub struct RenderPassResolve;
impl RenderPass for RenderPassResolve {
    type Params = SRes<DepthTexture>;

    fn describe(depth_texture: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc {
            attachments_depth: Some(RenderPassDescDepthAttachment {
                texture: Some(depth_texture.texture.id()),
                load: LoadOp::Clear(1.0),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn id() -> RenderPassId { 100 }
    fn label() -> &'static str { "pbr-resolve" }
}

#[derive(Default, Asset, Clone, TypePath)]
pub struct ResolveRenderPipelineAsset;
#[allow(dead_code)]
#[derive(Component)]
pub struct ResolveRenderPipeline(pub Handle<ResolveRenderPipelineAsset>);
pub struct GpuResolveRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for GpuResolveRenderPipeline {
    type SourceAsset = ResolveRenderPipelineAsset;
    type Param = (SRes<AssetServer>, SResMut<PipelineManager>);

    fn prepare_asset(
            _asset: Self::SourceAsset,
            (assets_server, pipeline_manager): &mut bevy::ecs::system::SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(GpuResolveRenderPipeline(pipeline_manager.create_render_pipeline(RenderPipelineDescriptor {
            label: "pbr-resolve",
            vert: Some(assets_server.load("core/render/resolve/vert.wgsl")),
            frag: Some(assets_server.load("core/render/resolve/frag.wgsl")),
            bind_group_layouts: vec![
                PbrRenderTextureBindGroup::layout(),
                DepthMSAATextureBindGroup::layout()
            ],
            depth: DepthDescriptor {
                enabled: true,
                write: true,
                compare: CompareFunction::Always,
                ..Default::default()
            },
            ..Default::default()
        })))
    }
}

pub struct SubRenderPassResolve;
impl RenderSubPass for SubRenderPassResolve {
    type Params = (SRes<RenderAssets<GpuResolveRenderPipeline>>, SRes<PostProcessingMesh>, SRes<DepthMSAATextureBindGroup>, SRes<PbrRenderTextureBindGroup>);

    fn describe(
        (pipeline, mesh, depth_msaa, pbr_texture): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(Some(pipeline.iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::Mesh(mesh.0.as_ref().map(|h| h.id())),
            SubPassCommand::BindGroup(0, pbr_texture.bind_group.clone()),
            SubPassCommand::BindGroup(1, depth_msaa.bind_group.clone()),
            SubPassCommand::DrawBatches(vec![DrawCommandsBatch {
                index_range: 0..6,
                ..Default::default()
            }])
        ])
    }

    fn label() -> &'static str { "pbr-resolve-main" }
}
