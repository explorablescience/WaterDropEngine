use bevy::{ecs::system::lifetimeless::{SRes, SResMut}, prelude::*};
use wde_camera::features::CameraFeatureRender;
use wde_renderer::prelude::*;

use crate::logic::{lights::LightsFeatureBuffer, textures::PbrDeferredTexturesLayout};


#[derive(Default, Asset, Clone, TypePath)]
pub(crate) struct PbrLightingRenderPipelineAsset;

#[allow(unused)]
#[derive(Component)]
pub(crate) struct PbrLightingRenderPipeline(pub Handle<PbrLightingRenderPipelineAsset>);
pub(crate) struct GpuPbrLightingRenderPipeline {
    pub cached_pipeline_index: CachedPipelineIndex
}
impl RenderAsset for GpuPbrLightingRenderPipeline {
    type SourceAsset = PbrLightingRenderPipelineAsset;
    type Param = (
        SRes<AssetServer>, SResMut<PipelineManager>, SRes<PbrDeferredTexturesLayout>,
        SRes<CameraFeatureRender>, SRes<LightsFeatureBuffer>
    );

    fn prepare_asset(
            asset: Self::SourceAsset,
            (
                assets_server, pipeline_manager,
                deferred_layout,
                camera_feature, lights_buffer
            ): &mut bevy::ecs::system::SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        // Get the deferred layout
        let deferred_layout_resolved = match &deferred_layout.deferred_layout_resolved {
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
            vert: Some(assets_server.load("core/render/pbr/lighting_vert.wgsl")),
            frag: Some(assets_server.load("core/render/pbr/lighting_frag.wgsl")),
            bind_group_layouts: vec![camera_feature.layout.clone(), deferred_layout_resolved.clone(), lights_layout.clone()],
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
