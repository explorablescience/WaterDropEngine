use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::render::{
    dependencies::{materials::TerrainMaterialArrays, terrain_buffer::TerrainBuffer},
    renderer_gpu::TerrainRendererGPU
};

#[derive(Default, Asset, Clone, TypePath, Debug)]
pub struct TerrainRenderPipelineAsset;
impl std::fmt::Display for TerrainRenderPipelineAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TerrainRenderPipelineAsset")
    }
}

#[allow(unused)]
#[derive(Component)]
pub struct TerrainRenderPipeline(pub Handle<TerrainRenderPipelineAsset>);
pub struct GpuTerrainRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for GpuTerrainRenderPipeline {
    type SourceAsset = TerrainRenderPipelineAsset;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<CameraRender>,
        SRes<TerrainMaterialArrays>,
        SRes<TerrainBuffer>
    );

    fn prepare(
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, camera, material_arrays, terrain_buffer): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(GpuTerrainRenderPipeline(
            pipeline_manager.create_render_pipeline(RenderPipelineDescriptor {
                label: "terrain",
                vert: Some(assets_server.load("core/render/terrain/render_terrain_vert.wgsl")),
                frag: Some(assets_server.load("core/render/terrain/render_terrain_frag.wgsl")),
                bind_group_layouts: vec![
                    camera.iter().next().map(|(_, c)| c.layout.clone()),
                    material_arrays.bind_group_layout.clone(),
                    Some(terrain_buffer.layout.clone()),
                    Some(TerrainRendererGPU::layout_render()),
                ],
                render_targets: Some(vec![
                    TextureFormat::R16Float,       // Depth
                    TextureFormat::Rgba8UnormSrgb, // Albedo
                    TextureFormat::Rgba16Float,    // Normal
                ]),
                depth: DepthDescriptor {
                    enabled: true,
                    ..Default::default()
                },
                sample_count: MSAA_SAMPLE_COUNT,
                ..Default::default()
            }, asset)?
        ))
    }

    fn label(&self) -> &str {
        "terrain"
    }
}
