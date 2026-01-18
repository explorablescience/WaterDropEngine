use bevy::{ecs::system::lifetimeless::{SRes, SResMut}, prelude::*};
use wde_camera::features::CameraFeatureRender;
use wde_renderer::{MSAA_SAMPLE_COUNT, prelude::*};

use crate::{assets::PbrMaterialAsset, passes::pbr_ssbo::PbrSsbo};

use super::{PbrDeferredTextures};


#[derive(Default, Asset, Clone, TypePath)]
pub(crate) struct PbrGBufferRenderPipelineAsset;

#[allow(unused)]
#[derive(Component)]
pub(crate) struct PbrGBufferRenderPipeline(pub Handle<PbrGBufferRenderPipelineAsset>);
pub(crate) struct GpuPbrGBufferRenderPipeline {
    pub cached_pipeline_index: CachedPipelineIndex
}
impl RenderAsset for GpuPbrGBufferRenderPipeline {
    type SourceAsset = PbrGBufferRenderPipelineAsset;
    type Param = (
        SRes<AssetServer>, SResMut<PipelineManager>,
        SRes<CameraFeatureRender>, SRes<RenderAssets<GpuMaterial<PbrMaterialAsset>>>, SRes<PbrSsbo>,
        SRes<PbrDeferredTextures>, SRes<RenderAssets<GpuTexture>>
    );

    fn prepare_asset(
            asset: Self::SourceAsset,
            (
                assets_server, pipeline_manager,
                camera_feature, materials, ssbo,
                defered_textures, textures
            ): &mut bevy::ecs::system::SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        // Get the defered textures
        let (depth, albedo, normal) =
            match (textures.get(&defered_textures.depth),
                   textures.get(&defered_textures.albedo),
                   textures.get(&defered_textures.normal)
            ) {
                (Some(depth), Some(albedo), Some(normal))
                    => (depth, albedo, normal),
                _ => return Err(PrepareAssetError::RetryNextUpdate(asset))
            };

        // Get the ssbo layout
        let ssbo_layout = match &ssbo.bind_group_layout {
            Some(layout) => layout,
            None => return Err(PrepareAssetError::RetryNextUpdate(asset))
        };

        // Get the material layout
        let material = match materials.iter().next() {
            Some((_, material)) => material,
            None => return Err(PrepareAssetError::RetryNextUpdate(asset))
        };

        // Create the pipeline
        let pipeline_desc = RenderPipelineDescriptor {
            label: "gbuffer-pbr",
            vert: Some(assets_server.load("core/render/pbr/gbuffer_vert.wgsl")),
            frag: Some(assets_server.load("core/render/pbr/gbuffer_frag.wgsl")),
            bind_group_layouts: vec![camera_feature.layout.clone(), ssbo_layout.clone(), material.bind_group_layout.clone()],
            depth: DepthStencilDescriptor {
                enabled: true,
                ..Default::default()
            },
            render_targets: Some(vec![
                depth.texture.format, albedo.texture.format, normal.texture.format
            ]),
            sample_count: MSAA_SAMPLE_COUNT,
            ..Default::default()
        };
        let cached_index = pipeline_manager.create_render_pipeline(pipeline_desc);

        Ok(GpuPbrGBufferRenderPipeline {
            cached_pipeline_index: cached_index
        })
    }

    fn label(&self) -> &str {
        "gbuffer-pbr"
    }
}
