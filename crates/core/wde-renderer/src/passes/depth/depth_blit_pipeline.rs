use bevy::{ecs::system::lifetimeless::{SRes, SResMut}, prelude::*};

use crate::{passes::depth::depth_texture_msaa::DepthMSAATextureLayout, prelude::*};


#[derive(Default, Asset, Clone, TypePath)]
pub struct DepthBlitRenderPipelineAsset;
#[allow(dead_code)]
#[derive(Component)]
pub struct DepthBlitRenderPipeline(pub Handle<DepthBlitRenderPipelineAsset>);
pub struct GpuDepthBlitRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for GpuDepthBlitRenderPipeline {
    type SourceAsset = DepthBlitRenderPipelineAsset;
    type Param = (SRes<AssetServer>, SResMut<PipelineManager>);

    fn prepare_asset(
            _asset: Self::SourceAsset,
            (assets_server, pipeline_manager): &mut bevy::ecs::system::SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(GpuDepthBlitRenderPipeline(pipeline_manager.create_render_pipeline(RenderPipelineDescriptor {
            label: "depth_blit",
            vert: Some(assets_server.load("core/render/depth_blit/vert.wgsl")),
            frag: Some(assets_server.load("core/render/depth_blit/frag.wgsl")),
            bind_group_layouts: vec![
                DepthMSAATextureLayout::layout()
            ],
            depth: DepthDescriptor {
                enabled: true,
                write: true,
                compare: CompareFunction::Always,
                ..Default::default()
            },
            render_targets: Some(vec![]),
            ..Default::default()
        })))
    }
}
