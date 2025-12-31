use bevy::{ecs::system::lifetimeless::{SRes, SResMut}, prelude::*};
use wde_wgpu::render_pipeline::DepthStencilDescriptor;
use crate::{assets::{PrepareAssetError, RenderAsset}, features::{CameraFeatureRender, LightsFeatureBuffer}, pipelines::{CachedPipelineIndex, PipelineManager, RenderPipelineDescriptor}, passes::depth::DepthTextureLayout};

use super::PbrDeferredTexturesLayout;


#[derive(Default, Asset, Clone, TypePath)]
pub struct PbrLightingRenderPipelineAsset;
#[derive(Component)]
pub struct PbrLightingRenderPipeline(pub Handle<PbrLightingRenderPipelineAsset>);
pub struct GpuPbrLightingRenderPipeline {
    pub cached_pipeline_index: CachedPipelineIndex
}
impl RenderAsset for GpuPbrLightingRenderPipeline {
    type SourceAsset = PbrLightingRenderPipelineAsset;
    type Param = (
        SRes<AssetServer>, SResMut<PipelineManager>, SRes<PbrDeferredTexturesLayout>,
        SRes<DepthTextureLayout>, SRes<CameraFeatureRender>, SRes<LightsFeatureBuffer>
    );

    fn prepare_asset(
            asset: Self::SourceAsset,
            (
                assets_server, pipeline_manager,
                deferred_layout, depth_texture_layout,
                camera_feature, lights_buffer
            ): &mut bevy::ecs::system::SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        // Get the depth layout
        let depth_layout = match &depth_texture_layout.layout {
            Some(layout) => layout,
            None => return Err(PrepareAssetError::RetryNextUpdate(asset))
        };

        // Get the deferred layout
        let deferred_layout = match &deferred_layout.deferred_layout {
            Some(layout) => layout,
            None => return Err(PrepareAssetError::RetryNextUpdate(asset))
        };

        // Get the lights buffer layout
        let lights_layout = match &lights_buffer.bind_group_layout {
            Some(layout) => layout,
            None => return Err(PrepareAssetError::RetryNextUpdate(asset))
        };

        // Create the pipeline
        let pipeline_desc = RenderPipelineDescriptor {
            label: "lighting-pbr",
            vert: Some(assets_server.load("pbr/lighting_vert.wgsl")),
            frag: Some(assets_server.load("pbr/lighting_frag.wgsl")),
            bind_group_layouts: vec![camera_feature.layout.clone(), depth_layout.clone(), deferred_layout.clone(), lights_layout.clone()],
            depth: DepthStencilDescriptor {
                enabled: false,
                ..Default::default()
            },
            render_targets: None,
            ..Default::default()
        };
        let cached_index = pipeline_manager.create_render_pipeline(pipeline_desc);

        Ok(GpuPbrLightingRenderPipeline {
            cached_pipeline_index: cached_index
        })
    }

    fn label(&self) -> &str {
        "lighting-pbr"
    }
}
