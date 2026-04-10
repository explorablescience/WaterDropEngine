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
    dependencies::{materials::TerrainMaterialsBinding, terrain_buffer::TerrainBufferBinding},
    renderer_gpu::TerrainTileBgRender
};

#[derive(Default, Asset, Clone, TypePath, Debug)]
pub(crate) struct TerrainRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for TerrainRenderPipeline {
    type SourceAsset = RenderPipelineAsset<TerrainRenderPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<CameraBinding>,
        SBinding<TerrainMaterialsBinding>,
        SBinding<TerrainBufferBinding>,
        SBinding<TerrainTileBgRender>
    );

    fn prepare(
        asset: Self::SourceAsset,
        (
            assets_server,
            pipeline_manager,
            camera,
            material_arrays,
            terrain_buffer,
            terrain_tile_bg_render
        ): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(TerrainRenderPipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "terrain",
                    vert: Some(assets_server.load("core/render/terrain/render_terrain_vert.wgsl")),
                    frag: Some(assets_server.load("core/render/terrain/render_terrain_frag.wgsl")),
                    bind_group_layouts: vec![
                        camera.iter().next().map(|(_, c)| c.layout.clone()),
                        material_arrays.iter().next().map(|(_, m)| m.layout.clone()),
                        terrain_buffer.iter().next().map(|(_, b)| b.layout.clone()),
                        terrain_tile_bg_render
                            .iter()
                            .next()
                            .map(|(_, b)| b.layout.clone()), // Take first one, they all have the same layout
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
                },
                asset
            )?
        ))
    }

    fn label(&self) -> &str {
        "terrain"
    }
}
