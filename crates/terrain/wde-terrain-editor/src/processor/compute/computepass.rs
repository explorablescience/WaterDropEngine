use wde_logger::prelude::*;

use bevy::prelude::*;
use wde_renderer::{prelude::*, wgpu_utils::CommandBuffer};
use wde_terrain::prelude::{
    CHUNK_RENDER_SUBDIVISIONS, CHUNK_SIZE, TerrainComputeArrayBg, TerrainRendererGPU
};

use crate::processor::{
    compute::pipeline::PaintComputePipeline,
    resources::commands_buffer::{CommandsBufferBinding, CommandsBufferDescription}
};

/// Push constants sent to the compute shader for each tile dispatch.
/// Size must match the WGSL struct (alignment rounds up to 32 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TileInfo {
    pub tile_idx: [f32; 2],
    pub tile_size: [f32; 2],
    pub tile_subdivisions: f32,
    pub tile_layer: u32,
    pub commands_count: u32,
    pub _pad: u32
}

/// One workgroup handles 64 pixels; the shader loops over all commands per pixel.
/// This is constant regardless of how many commands are queued.
const WORKGROUPS_PER_TILE: u32 = CHUNK_RENDER_SUBDIVISIONS * CHUNK_RENDER_SUBDIVISIONS / 64;

pub fn apply_paint_compute(
    render_instance: Res<RenderInstance>,
    commands_buffer: Binding<CommandsBufferBinding>,
    mut commands_buffer_desc: ResMut<CommandsBufferDescription>,
    pipeline_manager: Res<PipelineManager>,
    paint_pipeline: Res<RenderAssets<PaintComputePipeline>>,
    terrain_tiles: Res<TerrainRendererGPU>,
    compute_array_bg: Binding<TerrainComputeArrayBg>
) {
    if commands_buffer_desc.commands_count == 0 || commands_buffer_desc.dirty_chunks.is_empty() {
        return;
    }

    let pipeline = match paint_pipeline.iter().next() {
        Some((_, p)) => p,
        None => return
    };

    let render_instance = render_instance.0.read().unwrap();
    let mut command_buffer = CommandBuffer::new(&render_instance, "paint");
    {
        let mut compute_pass = command_buffer.create_compute_pass("paint");

        if let (CachedPipelineStatus::OkCompute(pipeline), Some((_, cmds_bg))) = (
            pipeline_manager.get_pipeline(pipeline.0),
            commands_buffer.iter().next()
        ) {
            if compute_pass.set_pipeline(pipeline).is_ok() {
                compute_pass.set_bind_group(0, &cmds_bg.bind_group);

                let Some((_, array_bg)) = compute_array_bg.iter().next() else {
                    return;
                };
                compute_pass.set_bind_group(1, &array_bg.bind_group);

                let commands_count = commands_buffer_desc.commands_count as u32;

                for tile_pos in &commands_buffer_desc.dirty_chunks {
                    let Some((&layer, tile)) = terrain_tiles
                        .pos_to_layer
                        .get_key_value(tile_pos)
                        .map(|(p, l)| (l, p))
                    else {
                        continue;
                    };

                    let push = TileInfo {
                        tile_idx: [tile.x as f32, tile.y as f32],
                        tile_size: [CHUNK_SIZE, CHUNK_SIZE],
                        tile_subdivisions: CHUNK_RENDER_SUBDIVISIONS as f32,
                        tile_layer: layer,
                        commands_count,
                        _pad: 0
                    };
                    compute_pass.set_push_constants(bytemuck::cast_slice(&[push]));

                    if let Err(e) = compute_pass.dispatch(WORKGROUPS_PER_TILE, 1, 1) {
                        error!(
                            "Failed to dispatch paint compute for tile {:?}: {:?}",
                            tile_pos, e
                        );
                    }
                }
            } else {
                error!("Failed to set paint compute pipeline");
            }
        }
    }

    command_buffer.submit(&render_instance);
    commands_buffer_desc.dirty_chunks.clear();
    commands_buffer_desc.commands_count = 0;
}
