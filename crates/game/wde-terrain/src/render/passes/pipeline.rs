use bevy::{ecs::system::lifetimeless::{SRes, SResMut}, prelude::*};
use wde_camera::features::CameraFeatureRender;
use wde_renderer::prelude::*;

use crate::{
    render::terrain::Terrain,
    render::materials::TerrainMaterialArrays,
};


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
        SRes<AssetServer>, SResMut<PipelineManager>, SRes<CameraFeatureRender>, SRes<Terrain>, SRes<TerrainMaterialArrays>
    );

    fn prepare_asset(
            asset: Self::SourceAsset,
            (
                assets_server, pipeline_manager, camera_feature, terrain, material_arrays
            ): &mut bevy::ecs::system::SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        // Get the terrain resource
        let terrain_layout = match terrain.tiles.first() {
            Some(tile) => {
                if let Some(layout) = &tile.bind_group_layout {
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
                terrain_layout.clone(),
                materials_layout.clone(),
            ],
            depth: DepthStencilDescriptor {
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
