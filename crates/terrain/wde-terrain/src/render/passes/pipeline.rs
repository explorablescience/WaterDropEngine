use bevy::{ecs::system::lifetimeless::{SRes, SResMut}, prelude::*};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::render::{dependencies::{materials::TerrainMaterialArrays, terrain_buffer::TerrainBuffer}, renderer_gpu::TerrainRendererGPU};


#[derive(Default, Asset, Clone, TypePath)]
pub(crate) struct TerrainRenderPipelineAsset;

#[allow(unused)]
#[derive(Component)]
pub(crate) struct TerrainRenderPipeline(pub Handle<TerrainRenderPipelineAsset>);
pub(crate) struct GpuTerrainRenderPipeline {
    pub cached_pipeline_index: CachedPipelineIndex
}
impl RenderAsset for GpuTerrainRenderPipeline {
    type SourceAsset = TerrainRenderPipelineAsset;
    type Param = (
        SRes<AssetServer>, SResMut<PipelineManager>, SRes<CameraFeatureRender>, SRes<TerrainRendererGPU>, SRes<TerrainMaterialArrays>, SRes<TerrainBuffer>
    );

    fn prepare_asset(
            asset: Self::SourceAsset,
            (
                assets_server, pipeline_manager, camera_feature, terrain_renderer, material_arrays, terrain_buffer
            ): &mut bevy::ecs::system::SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        // Check if the terrain renderer is ready
        if !terrain_renderer.ready {
            return Err(PrepareAssetError::RetryNextUpdate(asset));
        }

        // Get the terrain resource
        let terrain_layout = match terrain_renderer.tiles.first() {
            Some(tile) => {
                if let Some(layout) = &tile.render_bind_group_layout {
                    layout
                } else {
                    return Err(PrepareAssetError::RetryNextUpdate(asset));
                }
            },
            None => return Err(PrepareAssetError::RetryNextUpdate(asset)),
        };

        // Get the material arrays layout
        let materials_layout = match &material_arrays.bind_group_layout {
            Some(layout) => layout,
            None => return Err(PrepareAssetError::RetryNextUpdate(asset)),
        };

        // Create the pipeline
        let pipeline_desc = RenderPipelineDescriptor {
            label: "terrain",
            vert: Some(assets_server.load("core/render/terrain/render_terrain_vert.wgsl")),
            frag: Some(assets_server.load("core/render/terrain/render_terrain_frag.wgsl")),
            bind_group_layouts: vec![
                camera_feature.layout.clone(),
                materials_layout.clone(),
                terrain_buffer.layout.clone(),
                terrain_layout.clone()
            ],
            depth: DepthDescriptor {
                enabled: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let cached_index = pipeline_manager.create_render_pipeline(pipeline_desc);

        Ok(GpuTerrainRenderPipeline {
            cached_pipeline_index: cached_index
        })
    }

    fn label(&self) -> &str {
        "terrain"
    }
}
