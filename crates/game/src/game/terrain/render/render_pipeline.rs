use bevy::{ecs::system::lifetimeless::{SRes, SResMut}, prelude::*};
use wde_render::{assets::{PrepareAssetError, RenderAsset}, features::{CameraFeatureRender, LightsFeatureBuffer}, pipelines::{CachedPipelineIndex, PipelineManager, RenderPipelineDescriptor}};
use wde_wgpu::render_pipeline::DepthStencilDescriptor;


#[derive(Default, Asset, Clone, TypePath)]
pub struct MCRenderPipelineAsset;
#[derive(Component, Default)]
#[allow(dead_code)]
pub struct MCRenderPipeline(pub Handle<MCRenderPipelineAsset>);
pub struct GpuMCRenderPipeline {
    pub cached_pipeline_index: CachedPipelineIndex
}
impl RenderAsset for GpuMCRenderPipeline {
    type SourceAsset = MCRenderPipelineAsset;
    type Param = (
        SRes<AssetServer>, SResMut<PipelineManager>,
        SRes<CameraFeatureRender>, SRes<LightsFeatureBuffer>
    );

    fn prepare_asset(
            asset: Self::SourceAsset,
            (
                assets_server, pipeline_manager,
                camera_feature, lights_buffer
            ): &mut bevy::ecs::system::SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {

        // Get the lights buffer layout
        let lights_layout = match &lights_buffer.bind_group_layout {
            Some(layout) => layout,
            None => return Err(PrepareAssetError::RetryNextUpdate(asset))
        };

        // Create the pipeline
        let pipeline_desc = RenderPipelineDescriptor {
            label: "marching-cubes",
            vert: Some(assets_server.load("marching-cubes/render.vert.wgsl")),
            frag: Some(assets_server.load("marching-cubes/render.frag.wgsl")),
            bind_group_layouts: vec![camera_feature.layout.clone(), lights_layout.clone()],
            depth: DepthStencilDescriptor {
                enabled: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let cached_index = pipeline_manager.create_render_pipeline(pipeline_desc);

        Ok(GpuMCRenderPipeline {
            cached_pipeline_index: cached_index
        })
    }

    fn label(&self) -> &str {
        "marching-cubes-render"
    }
}
