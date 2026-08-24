use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_camera::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

use crate::render::{
    dependencies::{materials::TerrainMaterialsBinding, terrain_buffer::TerrainBufferBinding},
    renderer_gpu::TerrainChunkArrayBg
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
        SBinding<TerrainChunkArrayBg>
    );

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, camera, material_arrays, terrain_buffer, chunk_array): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(TerrainRenderPipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "terrain",
                    vert: Some(assets_server.load("core/render/terrain/render_terrain.vert.wgsl")),
                    frag: Some(assets_server.load("core/render/terrain/render_terrain.frag.wgsl")),
                    bind_group_layouts: vec![
                        camera.iter().next().map(|(_, c)| c.layout.clone()),
                        material_arrays.iter().next().map(|(_, m)| m.layout.clone()),
                        terrain_buffer.iter().next().map(|(_, b)| b.layout.clone()),
                        chunk_array.iter().next().map(|(_, b)| b.layout.clone()),
                    ],
                    render_targets: Some(vec![
                        // Same order as the PbrDeferredTextures
                        PbrTextureFormat::DEPTH,  // Depth
                        PbrTextureFormat::ALBEDO, // Albedo
                        PbrTextureFormat::NORMAL, // Normal
                        PbrTextureFormat::AO,     // AO
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
