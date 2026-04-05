use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::logic::{deferred_textures::PbrDeferredTexturesLayout, lights::LightsFeatureBuffer};

#[derive(Default, Asset, Clone, TypePath)]
pub struct PbrLightingRenderPipelineAsset;

#[allow(unused)]
#[derive(Component)]
pub struct PbrLightingRenderPipeline(pub Handle<PbrLightingRenderPipelineAsset>);
pub struct GpuPbrLightingRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for GpuPbrLightingRenderPipeline {
    type SourceAsset = PbrLightingRenderPipelineAsset;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SRes<CameraFeatureRender>
    );

    fn prepare(
        _asset: Self::SourceAsset,
        (assets_server, pipeline_manager, camera_feature): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(GpuPbrLightingRenderPipeline(
            pipeline_manager.create_render_pipeline(RenderPipelineDescriptor {
                label: "lighting-pbr",
                vert: Some(assets_server.load("core/render/pbr/lighting_vert.wgsl")),
                frag: Some(assets_server.load("core/render/pbr/lighting_frag.wgsl")),
                bind_group_layouts: vec![
                    camera_feature.layout.clone(),
                    PbrDeferredTexturesLayout::layout(),
                    LightsFeatureBuffer::layout(),
                ],
                depth: DepthDescriptor {
                    enabled: false,
                    ..default()
                },
                sample_count: MSAA_SAMPLE_COUNT,
                ..default()
            })
        ))
    }
}
