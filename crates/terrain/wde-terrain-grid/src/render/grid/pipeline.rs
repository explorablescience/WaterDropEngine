use bevy::{ecs::system::lifetimeless::{SRes, SResMut}, prelude::*};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::render::grid::buffers::TerrainGridBuffer;


#[derive(Default, Asset, Clone, TypePath)]
pub(crate) struct TerrainGridRenderPipelineAsset;

#[allow(unused)]
#[derive(Component)]
pub(crate) struct TerrainGridRenderPipeline(pub Handle<TerrainGridRenderPipelineAsset>);
pub(crate) struct GpuTerrainGridRenderPipeline {
    pub cached_pipeline_index: CachedPipelineIndex
}
impl RenderAsset for GpuTerrainGridRenderPipeline {
    type SourceAsset = TerrainGridRenderPipelineAsset;
    type Param = (
        SRes<AssetServer>, SResMut<PipelineManager>, SRes<CameraFeatureRender>, SRes<TerrainGridBuffer>
    );

    fn prepare_asset(
            _asset: Self::SourceAsset,
            (
                assets_server, pipeline_manager, camera_feature, terrain_grid_buffer
            ): &mut bevy::ecs::system::SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        // Create the pipeline
        let pipeline_desc = RenderPipelineDescriptor {
            label: "terrain-grid",
            vert: Some(assets_server.load("core/render/terrain/render_grid_vert.wgsl")),
            frag: Some(assets_server.load("core/render/terrain/render_grid_frag.wgsl")),
            bind_group_layouts: vec![
                camera_feature.layout.clone(),
                terrain_grid_buffer.layout.clone()
            ],
            depth: DepthDescriptor {
                enabled: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let cached_index = pipeline_manager.create_render_pipeline(pipeline_desc);

        Ok(GpuTerrainGridRenderPipeline {
            cached_pipeline_index: cached_index
        })
    }

    fn label(&self) -> &str {
        "terrain-grid"
    }
}
