use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::render::grid::buffers::TerrainGridBuffer;

#[derive(Default, Asset, Clone, TypePath, Debug)]
pub struct TerrainGridRenderPipelineAsset;
impl std::fmt::Display for TerrainGridRenderPipelineAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TerrainGridRenderPipelineAsset")
    }
}

#[allow(unused)]
#[derive(Component)]
pub struct TerrainGridRenderPipeline(pub Handle<TerrainGridRenderPipelineAsset>);
pub struct GpuTerrainGridRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for GpuTerrainGridRenderPipeline {
    type SourceAsset = TerrainGridRenderPipelineAsset;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<CameraRender>,
        SRes<TerrainGridBuffer>
    );

    fn prepare(
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, camera, terrain_grid_buffer): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(GpuTerrainGridRenderPipeline(
            pipeline_manager.create_render_pipeline(RenderPipelineDescriptor {
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
                    camera.iter().next().map(|(_, c)| c.layout.clone()),
                    Some(terrain_grid_buffer.layout.clone()),
                ],
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
        "terrain-grid"
    }
}
