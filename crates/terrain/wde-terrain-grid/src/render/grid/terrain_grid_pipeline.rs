use bevy::{ecs::system::{SystemParamItem, lifetimeless::{SRes, SResMut}}, prelude::*};
use wde_camera::prelude::*;
use wde_renderer::{MSAA_SAMPLE_COUNT, prelude::*};

use crate::render::grid::buffers::TerrainGridBuffer;


#[derive(Default, Asset, Clone, TypePath)]
pub struct TerrainGridRenderPipelineAsset;

#[allow(unused)]
#[derive(Component)]
pub struct TerrainGridRenderPipeline(pub Handle<TerrainGridRenderPipelineAsset>);
pub struct GpuTerrainGridRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for GpuTerrainGridRenderPipeline {
    type SourceAsset = TerrainGridRenderPipelineAsset;
    type Param = (
        SRes<AssetServer>, SResMut<PipelineManager>, SRes<CameraFeatureRender>, SRes<TerrainGridBuffer>
    );

    fn prepare_asset(
            _asset: Self::SourceAsset,
            (assets_server, pipeline_manager, camera_feature, terrain_grid_buffer): &mut SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(GpuTerrainGridRenderPipeline(pipeline_manager.create_render_pipeline(RenderPipelineDescriptor {
            label: "terrain-grid",
            vert: Some(assets_server.load("core/render/terrain/render_grid_vert.wgsl")),
            frag: Some(assets_server.load("core/render/terrain/render_grid_frag.wgsl")),
            fragment_blend: Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add
                },
                alpha: BlendComponent::OVER
            }),
            bind_group_layouts: vec![
                camera_feature.layout.clone(),
                terrain_grid_buffer.layout.clone()
            ],
            depth: DepthDescriptor {
                enabled: true,
                ..Default::default()
            },
            sample_count: MSAA_SAMPLE_COUNT,
            ..Default::default()
        })))
    }

    fn label(&self) -> &str {
        "terrain-grid"
    }
}
