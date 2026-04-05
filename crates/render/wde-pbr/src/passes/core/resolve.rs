use wde_renderer::prelude::*;
use bevy::{ecs::system::{SystemParamItem, lifetimeless::{SRes, SResMut}}, prelude::*};

use crate::logic::render_texture::PbrRenderTextureBindGroup;

pub struct RenderPassResolve;
impl RenderPass for RenderPassResolve {
    type Params = ();

    fn describe(_: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc::default()
    }
    fn id() -> RenderPassId { 100 }
    fn label() -> &'static str { "pbr-resolve" }
}

#[derive(TypePath, Default, Clone)]
pub struct ResolveRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for ResolveRenderPipeline {
    type SourceAsset = RenderPipelineAsset<ResolveRenderPipeline>;
    type Params = (SRes<AssetServer>, SResMut<PipelineManager>);

    fn prepare(
            _asset: Self::SourceAsset,
            (assets_server, pipeline_manager): &mut bevy::ecs::system::SystemParamItem<Self::Params>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(ResolveRenderPipeline(pipeline_manager.create_render_pipeline(RenderPipelineDescriptor {
            label: "pbr-resolve",
            vert: Some(assets_server.load("core/render/resolve/vert.wgsl")),
            frag: Some(assets_server.load("core/render/resolve/frag.wgsl")),
            bind_group_layouts: vec![
                PbrRenderTextureBindGroup::layout()
            ],
            ..Default::default()
        })))
    }
}

pub struct SubRenderPassResolve;
impl RenderSubPass for SubRenderPassResolve {
    type Params = (SRes<RenderAssets<ResolveRenderPipeline>>, SRes<PostProcessingMesh>, SRes<PbrRenderTextureBindGroup>);

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
            }])
        ])
    }

    fn label() -> &'static str { "pbr-resolve-main" }
}
