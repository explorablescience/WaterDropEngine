use bevy::{ecs::system::lifetimeless::{SRes, SResMut}, prelude::*};
use wde_renderer::prelude::*;
use wde_terrain::prelude::TerrainRendererGPU;

use crate::processor::{compute::computepass::TileInfo, resources::commands_buffer::CommandsBuffer};


#[derive(Default, Asset, Clone, TypePath)]
pub(crate) struct PaintComputePipelineAsset;

#[allow(unused)]
#[derive(Component)]
pub(crate) struct PaintComputePipeline(pub Handle<PaintComputePipelineAsset>);
pub(crate) struct GpuPaintComputePipeline {
    pub cached_pipeline_index: CachedPipelineIndex
}
impl RenderAsset for GpuPaintComputePipeline {
    type SourceAsset = PaintComputePipelineAsset;
    type Param = (
        SRes<AssetServer>, SResMut<PipelineManager>, SRes<CommandsBuffer>, SRes<TerrainRendererGPU>
    );

    fn prepare_asset(
            asset: Self::SourceAsset,
            (
                assets_server, pipeline_manager, commands_buffer, terrain_tiles
            ): &mut bevy::ecs::system::SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        // Get the layout of the first tile to create the pipeline layout
        let tiles_layout = match terrain_tiles.tiles.first() {
            Some(tile) => match tile.compute_bind_group_layout.clone() {
                Some(layout) => layout,
                None => return Err(PrepareAssetError::RetryNextUpdate(asset))
            },
            None => return Err(PrepareAssetError::RetryNextUpdate(asset))
        };

        // Create the pipeline
        let pipeline_desc = ComputePipelineDescriptor {
            label: "paint_compute",
            comp: Some(assets_server.load("core/compute/terrain_editor/paint_compute.wgsl")),
            bind_group_layouts: vec![
                commands_buffer.layout.clone(),
                tiles_layout
            ],
            push_constants: vec![PushConstantDescriptor {
                stages: ShaderStages::COMPUTE,
                offset: 0,
                size: std::mem::size_of::<TileInfo>() as u32
            }]
        };
        let cached_index = pipeline_manager.create_compute_pipeline(pipeline_desc);

        Ok(GpuPaintComputePipeline {
            cached_pipeline_index: cached_index
        })
    }

    fn label(&self) -> &str {
        "paint_compute"
    }
}
