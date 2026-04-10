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

#[derive(Clone, Default, Debug, TypePath)]
pub struct TerrainGridRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for TerrainGridRenderPipeline {
    type SourceAsset = RenderPipelineAsset<TerrainGridRenderPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SRenderBinding<CameraBinding>,
        SBindingOld<TerrainGridBuffer>
    );

    fn prepare(
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, camera, terrain_grid_buffer): &mut SystemParamItem<
            Self::Params
        >
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(TerrainGridRenderPipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
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
                        terrain_grid_buffer
                            .iter()
                            .next()
                            .map(|(_, c)| c.layout.clone()),
                    ],
                    depth: DepthDescriptor {
                        enabled: true,
                        ..Default::default()
                    },
                    sample_count: MSAA_SAMPLE_COUNT,
                    ..Default::default()
                },
                asset
            )?
        ))
    }

    fn label(&self) -> &str {
        "terrain-grid"
    }
}
