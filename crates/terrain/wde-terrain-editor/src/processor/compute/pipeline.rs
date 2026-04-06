use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_renderer::prelude::*;
use wde_terrain::prelude::TerrainRendererGPU;

use crate::processor::{
    compute::computepass::TileInfo, resources::commands_buffer::CommandsBuffer
};

#[derive(Default, Asset, Clone, TypePath)]
pub(crate) struct PaintComputePipelineAsset;

#[allow(unused)]
#[derive(Component)]
pub(crate) struct PaintComputePipeline(pub Handle<PaintComputePipelineAsset>);
pub(crate) struct GpuPaintComputePipeline(pub CachedPipelineIndex);
impl RenderAsset for GpuPaintComputePipeline {
    type SourceAsset = PaintComputePipelineAsset;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SRes<CommandsBuffer>
    );

    fn prepare(
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, commands_buffer): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(GpuPaintComputePipeline(
            pipeline_manager.create_compute_pipeline(ComputePipelineDescriptor {
                label: "paint_compute",
                comp: Some(assets_server.load("core/compute/terrain_editor/paint_compute.wgsl")),
                bind_group_layouts: vec![
                    Some(commands_buffer.layout.clone()),
                    Some(TerrainRendererGPU::layout_compute()),
                ],
                push_constants: vec![PushConstantDescriptor {
                    stages: ShaderStages::COMPUTE,
                    offset: 0,
                    size: std::mem::size_of::<TileInfo>() as u32
                }]
            }, asset)?
        ))
    }

    fn label(&self) -> &str {
        "paint_compute"
    }
}
