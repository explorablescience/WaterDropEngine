use bevy::{ecs::system::{SystemParamItem, lifetimeless::{SRes, SResMut}}, prelude::*};
use wde_camera::prelude::*;
use wde_renderer::{MSAA_SAMPLE_COUNT, prelude::*};

use crate::render::{dependencies::{materials::TerrainMaterialArrays, terrain_buffer::TerrainBuffer}, renderer_gpu::TerrainRendererGPU};


#[derive(Default, Asset, Clone, TypePath)]
pub struct TerrainRenderPipelineAsset;

#[allow(unused)]
#[derive(Component)]
pub struct TerrainRenderPipeline(pub Handle<TerrainRenderPipelineAsset>);
pub struct GpuTerrainRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for GpuTerrainRenderPipeline {
    type SourceAsset = TerrainRenderPipelineAsset;
    type Param = (
        SRes<AssetServer>, SResMut<PipelineManager>, SRes<CameraFeatureRender>, SRes<TerrainMaterialArrays>, SRes<TerrainBuffer>
    );

    fn prepare_asset(
            asset: Self::SourceAsset,
            (assets_server, pipeline_manager, camera_feature, material_arrays, terrain_buffer): &mut SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let materials_layout = match &material_arrays.bind_group_layout {
            Some(layout) => layout,
            None => return Err(PrepareAssetError::RetryNextUpdate(asset)),
        };

        Ok(GpuTerrainRenderPipeline(pipeline_manager.create_render_pipeline(RenderPipelineDescriptor {
            label: "terrain",
            vert: Some(assets_server.load("core/render/terrain/render_terrain_vert.wgsl")),
            frag: Some(assets_server.load("core/render/terrain/render_terrain_frag.wgsl")),
            bind_group_layouts: vec![
                camera_feature.layout.clone(),
                materials_layout.clone(),
                terrain_buffer.layout.clone(),
                TerrainRendererGPU::layout_render()
            ],
            render_targets: Some(vec![
                TextureFormat::R16Float,       // Depth
                TextureFormat::Rgba8UnormSrgb, // Albedo
                TextureFormat::Rgba16Float     // Normal
            ]),
            depth: DepthDescriptor {
                enabled: true,
                ..Default::default()
            },
            sample_count: MSAA_SAMPLE_COUNT,
            ..Default::default()
        })))
    }

    fn label(&self) -> &str {
        "terrain"
    }
}
