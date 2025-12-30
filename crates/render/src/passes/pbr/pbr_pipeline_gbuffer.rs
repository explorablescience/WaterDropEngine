use bevy::{ecs::system::lifetimeless::{SRes, SResMut}, prelude::*};
use wde_wgpu::render_pipeline::DepthStencilDescriptor;
use crate::{assets::{materials::PbrMaterialAsset, GpuMaterial, GpuTexture, PrepareAssetError, RenderAsset, RenderAssets}, features::CameraFeatureRender, pipelines::{CachedPipelineIndex, PipelineManager, RenderPipelineDescriptor}};

use super::{PbrDeferredTextures, PbrSsbo};


#[derive(Default, Asset, Clone, TypePath)]
pub struct PbrGBufferRenderPipelineAsset;
#[derive(Component)]
pub struct PbrGBufferRenderPipeline(pub Handle<PbrGBufferRenderPipelineAsset>);
pub struct GpuPbrGBufferRenderPipeline {
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
        let (albedo, normal, material_tex) =
            match (textures.get(&defered_textures.albedo),
                   textures.get(&defered_textures.normal), textures.get(&defered_textures.material)
            ) {
                (Some(albedo), Some(normal), Some(material_tex))
                    => (albedo, normal, material_tex),
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
            vert: Some(assets_server.load("pbr/gbuffer_vert.wgsl")),
            frag: Some(assets_server.load("pbr/gbuffer_frag.wgsl")),
            bind_group_layouts: vec![camera_feature.layout.clone(), ssbo_layout.clone(), material.bind_group_layout.clone()],
            depth: DepthStencilDescriptor {
                enabled: true,
                ..Default::default()
            },
            render_targets: Some(vec![
                albedo.texture.format, normal.texture.format, material_tex.texture.format
            ]),
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
