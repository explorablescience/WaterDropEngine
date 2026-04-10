use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_renderer::prelude::*;
use wde_terrain::render::renderer_gpu::TerrainTileBgCompute;

use crate::processor::{
    compute::computepass::TileInfo, resources::commands_buffer::CommandsBuffer
};

#[derive(Default, Asset, Clone, TypePath)]
pub(crate) struct PaintComputePipeline(pub CachedPipelineIndex);
impl RenderAsset for PaintComputePipeline {
    type SourceAsset = RenderPipelineAsset<Self>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBindingOld<CommandsBuffer>,
        SBindingOld<TerrainTileBgCompute>
    );

    fn prepare(
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, commands_buffer, terrain_tile_bg_compute): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(PaintComputePipeline(
            pipeline_manager.create_compute_pipeline(
                ComputePipelineDescriptor {
                    label: "paint_compute",
                    comp: Some(
                        assets_server.load("core/compute/terrain_editor/paint_compute.wgsl")
                    ),
                    bind_group_layouts: vec![
                        commands_buffer.iter().next().map(|(_, b)| b.layout.clone()),
                        terrain_tile_bg_compute
                            .iter()
                            .next()
                            .map(|(_, b)| b.layout.clone()), // Take first one, they all have the same layout
                    ],
                    push_constants: vec![PushConstantDescriptor {
                        stages: ShaderStages::COMPUTE,
                        offset: 0,
                        size: std::mem::size_of::<TileInfo>() as u32
                    }]
                },
                asset
            )?
        ))
    }

    fn label(&self) -> &str {
        "paint_compute"
    }
}
