use bevy::{ecs::system::lifetimeless::{SRes, SResMut}, prelude::*};

use crate::{passes::depth_msaa::DepthMSAATextureLayout, prelude::*};


#[derive(Default, Asset, Clone, TypePath)]
pub(crate) struct DepthBlitRenderPipelineAsset;

#[allow(dead_code)]
#[derive(Component)]
pub(crate) struct DepthBlitRenderPipeline(pub Handle<DepthBlitRenderPipelineAsset>);
pub(crate) struct GpuDepthBlitRenderPipeline {
    pub cached_pipeline_index: CachedPipelineIndex
}
impl RenderAsset for GpuDepthBlitRenderPipeline {
    type SourceAsset = DepthBlitRenderPipelineAsset;
    type Param = (SRes<AssetServer>, SResMut<PipelineManager>, SRes<DepthMSAATextureLayout>);

    fn prepare_asset(
            asset: Self::SourceAsset,
            (assets_server, pipeline_manager, depth_msaa_layout): &mut bevy::ecs::system::SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        // Get the depth msaa layout
        let depth_msaa_layout = match &depth_msaa_layout.layout {
            Some(layout) => layout,
            None => return Err(PrepareAssetError::RetryNextUpdate(asset))
        };

        // Create the pipeline
        let pipeline_desc = RenderPipelineDescriptor {
            label: "depth_blit",
            vert: Some(assets_server.load("core/render/depth_blit/vert.wgsl")),
            frag: Some(assets_server.load("core/render/depth_blit/frag.wgsl")),
            bind_group_layouts: vec![depth_msaa_layout.clone()],
            depth: DepthDescriptor {
                enabled: true,
                write: true,
                compare: CompareFunction::Always,
                ..Default::default()
            },
            render_targets: Some(vec![]),
            ..Default::default()
        };
        let cached_index = pipeline_manager.create_render_pipeline(pipeline_desc);

        Ok(GpuDepthBlitRenderPipeline {
            cached_pipeline_index: cached_index
        })
    }

    fn label(&self) -> &str {
        "depth_blit"
    }
}
