use wde_logger::prelude::*;

use bevy::prelude::*;
use wde_renderer::{prelude::*, wgpu_utils::CommandBuffer};
use wde_terrain::{prelude::*, render::renderer_gpu::TerrainTileBgCompute};

use crate::processor::{
    compute::pipeline::PaintComputePipeline,
    resources::commands_buffer::{CommandsBufferBinding, CommandsBufferDescription}
};

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TileInfo {
    pub tile_idx: [f32; 2],
    pub tile_size: [f32; 2],
    pub tile_subdivisions: f32
}

pub fn apply_paint_compute(
    render_instance: Res<RenderInstance>,
    commands_buffer: Binding<CommandsBufferBinding>,
    mut commands_buffer_desc: ResMut<CommandsBufferDescription>,
    pipeline_manager: Res<PipelineManager>,
    paint_pipeline: Res<RenderAssets<PaintComputePipeline>>,
    terrain_tiles: Res<TerrainRendererGPU>,
    terrain_tile_bg_compute: Binding<TerrainTileBgCompute>
) {
    // Check if we should render
    if commands_buffer_desc.commands_count == 0 || commands_buffer_desc.dirty_chunks.is_empty() {
        return;
    }

    // Get the pipeline
    let pipeline = match paint_pipeline.iter().next() {
        Some((_, pipeline)) => pipeline,
        None => return
    };

    // Create the render pass
    let render_instance = render_instance.0.read().unwrap();
    let mut command_buffer = CommandBuffer::new(&render_instance, "paint");
    {
        let mut compute_pass = command_buffer.create_compute_pass("paint");

        if let (CachedPipelineStatus::OkCompute(pipeline), Some((_, commands_buffer))) = (
            pipeline_manager.get_pipeline(pipeline.0),
            &commands_buffer.iter().next()
        ) {
            if compute_pass.set_pipeline(pipeline).is_ok() {
                // Set the commands buffer bind group
                compute_pass.set_bind_group(0, &commands_buffer.bind_group);

                // Raw number of workgroups to dispatch
                const MAX_PER_DISPATCH: u32 = 65535; // Max workgroups per dispatch for most GPUs
                let count_raw = commands_buffer_desc.commands_count as u32
                    * CHUNK_RENDER_SUBDIVISIONS
                    * CHUNK_RENDER_SUBDIVISIONS
                    / 64; // workgroup size
                let it = ops::ceil(count_raw as f32 / MAX_PER_DISPATCH as f32);
                let last_it_count = count_raw % MAX_PER_DISPATCH;

                for i in 0..it as u32 {
                    // Calculate the number of workgroups to dispatch for this iteration
                    let count = if i == it as u32 - 1 {
                        last_it_count
                    } else {
                        MAX_PER_DISPATCH
                    };

                    // Dispatch the compute shader
                    for tile_id in &commands_buffer_desc.dirty_chunks {
                        // Get the tile index from the position
                        let tile_index = match terrain_tiles.pos_to_tile.get(tile_id) {
                            Some(index) => *index,
                            None => continue
                        };
                        let tile = &terrain_tiles.tiles[tile_index];

                        // Set push constants
                        let push_constants = TileInfo {
                            tile_idx: [tile.position.x as f32, tile.position.y as f32],
                            tile_size: [CHUNK_SIZE, CHUNK_SIZE],
                            tile_subdivisions: CHUNK_RENDER_SUBDIVISIONS as f32
                        };
                        compute_pass.set_push_constants(bytemuck::cast_slice(&[push_constants]));

                        // Set the bind group for the tile
                        let tile_bind_group = match &tile.compute_bind_group {
                            Some(bind_group) => match terrain_tile_bg_compute.get(bind_group) {
                                Some(bg) => &bg.bind_group,
                                None => continue
                            },
                            None => continue
                        };
                        compute_pass.set_bind_group(1, tile_bind_group);

                        // Dispatch the compute shader for the tile
                        if let Err(e) = compute_pass.dispatch(count, 1, 1) {
                            error!(
                                "Failed to dispatch paint compute shader for tile {:?}: {:?}",
                                tile_id, e
                            );
                        }
                    }
                }
            } else {
                error!("Failed to set paint compute pipeline");
            }
        }
    }

    // Submit the command buffer
    command_buffer.submit(&render_instance);

    // Reset should_render to false and clear the commands buffer for the next frame
    commands_buffer_desc.dirty_chunks.clear();
    commands_buffer_desc.commands_count = 0;
}
