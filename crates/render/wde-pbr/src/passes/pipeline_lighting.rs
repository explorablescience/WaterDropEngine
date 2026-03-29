use bevy::{ecs::system::{SystemParamItem, lifetimeless::{SRes, SResMut}}, prelude::*};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::logic::{lights::LightsFeatureBuffer, textures::PbrDeferredTexturesLayout};


#[derive(Default, Asset, Clone, TypePath)]
pub(crate) struct PbrLightingRenderPipelineAsset;

#[allow(unused)]
#[derive(Component)]
pub(crate) struct PbrLightingRenderPipeline(pub Handle<PbrLightingRenderPipelineAsset>);
pub(crate) struct GpuPbrLightingRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for GpuPbrLightingRenderPipeline {
    type SourceAsset = PbrLightingRenderPipelineAsset;
    type Param = (SRes<AssetServer>, SResMut<PipelineManager>, SRes<CameraFeatureRender>);

    fn prepare_asset(
            _asset: Self::SourceAsset,
            (assets_server, pipeline_manager, camera_feature): &mut SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(GpuPbrLightingRenderPipeline(pipeline_manager.create_render_pipeline(RenderPipelineDescriptor {
            label: "lighting-pbr",
            vert: Some(assets_server.load("core/render/pbr/lighting_vert.wgsl")),
            frag: Some(assets_server.load("core/render/pbr/lighting_frag.wgsl")),
            bind_group_layouts: vec![
                camera_feature.layout.clone(),
                PbrDeferredTexturesLayout::layout(),
                LightsFeatureBuffer::layout()
            ],
            depth: DepthDescriptor {
                enabled: false,
                ..default()
            },
            ..default()
        })))
    }
}
