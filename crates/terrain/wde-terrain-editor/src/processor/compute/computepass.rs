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
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TileInfo {
    pub tile_idx: [f32; 2],
    pub tile_size: [f32; 2],
    pub tile_subdivisions: f32,
    pub tile_layer: u32
}

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

                // Set the single array bind group once for all tile dispatches.
                if let Some((_, array_bg)) = compute_array_bg.iter().next() {
                    compute_pass.set_bind_group(1, &array_bg.bind_group);
                } else {
                    return;
                }

                const MAX_PER_DISPATCH: u32 = 65535;
                let count_raw = commands_buffer_desc.commands_count as u32
                    * CHUNK_RENDER_SUBDIVISIONS
                    * CHUNK_RENDER_SUBDIVISIONS
                    / 64;
                let iterations = ops::ceil(count_raw as f32 / MAX_PER_DISPATCH as f32) as u32;
                let last_count = count_raw % MAX_PER_DISPATCH;

                for i in 0..iterations {
                    let count = if i == iterations - 1 {
                        last_count
                    } else {
                        MAX_PER_DISPATCH
                    };

                    for tile_pos in &commands_buffer_desc.dirty_chunks {
                        let layer = match terrain_tiles.pos_to_layer.get(tile_pos) {
                            Some(&l) => l,
                            None => continue
                        };
                        let tile = match terrain_tiles
                            .pos_to_layer
                            .get_key_value(tile_pos)
                            .map(|(p, _)| p)
                        {
                            Some(p) => p,
                            None => continue
                        };

                        let push = TileInfo {
                            tile_idx: [tile.x as f32, tile.y as f32],
                            tile_size: [CHUNK_SIZE, CHUNK_SIZE],
                            tile_subdivisions: CHUNK_RENDER_SUBDIVISIONS as f32,
                            tile_layer: layer
                        };
                        compute_pass.set_push_constants(bytemuck::cast_slice(&[push]));

                        if let Err(e) = compute_pass.dispatch(count, 1, 1) {
                            error!(
                                "Failed to dispatch paint compute for tile {:?}: {:?}",
                                tile_pos, e
                            );
                        }
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
