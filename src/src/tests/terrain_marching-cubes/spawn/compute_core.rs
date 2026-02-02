use bevy::prelude::*;
use wde_render::{assets::{Buffer, GpuBuffer, RenderAssets}, core::{DeviceLimits, RenderInstance, extract_macros::ExtractWorld}, pipelines::{CachedPipelineStatus, PipelineManager}};
use wde_wgpu::{bind_group::BindGroup, buffer::BufferUsage, command_buffer::CommandBuffer};

use crate::terrain::{mc_chunk::{MCActiveChunk, MCChunksListMain, MCChunksListRender, MCLoadingChunk, MCPendingChunk, MCRegisteredChunk}, mc_compute_main::{GpuMCDescription, MCComputeHandlerGPU, MCTerrainNoiseParameters}, MC_MAX_CHUNKS_PROCESS_PER_FRAME, MC_MAX_POINTS};

use super::compute_pipeline::GpuMCComputePipelineSpawn;

pub struct MCComputePointsCore;
impl MCComputePointsCore {
    /** Process the spawn events to generate the chunks on the render thread. */
    pub fn extract(
        chunks_list_main: ExtractWorld<Res<MCChunksListMain>>,
        mut chunks_list_render: ResMut<MCChunksListRender>,
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        device_limits: Res<DeviceLimits>,
        (query_registered, query_loading, query_pending, query_active): (
            Query<(Entity, &MCRegisteredChunk)>,
            Query<(Entity, &MCLoadingChunk)>,
            Query<(Entity, &MCPendingChunk)>,
            Query<(Entity, &MCActiveChunk)>
        )
    ) {
        // Add the new chunks from the main thread to the render thread
        let max_buffer_size = device_limits.0.max_storage_buffer_binding_size as usize;
        for (index, desc) in chunks_list_main.new_chunks.iter() {
            chunks_list_render.chunks.insert(*index, desc.clone());
            
            // Create the points buffer
            let points_gpu = Buffer {
                label: format!("marching-cubes-points-{:?}", desc.index),
                size: std::cmp::min(std::mem::size_of::<[f32; 4]>() * MC_MAX_POINTS as usize, max_buffer_size),
                usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                content: None
            };

            commands.spawn((
                MCRegisteredChunk {
                    index: *index,
                    points_gpu: asset_server.add(points_gpu),
                    points_gpu_group: None,
                },
                desc.clone()
            ));
        }

        // Remove the old chunks from the render thread
        for index in chunks_list_main.delete_chunks.iter() {
            // Remove the chunk from the list of all chunks
            chunks_list_render.chunks.remove(index);

            // Remove the chunk from the registered chunks
            for (entity, chunk) in query_registered.iter() {
                if chunk.index == *index {
                    commands.entity(entity).despawn();
                }
            }
            for (entity, chunk) in query_loading.iter() {
                if chunk.index == *index {
                    commands.entity(entity).despawn();
                }
            }
            for (entity, chunk) in query_pending.iter() {
                if chunk.index == *index {
                    commands.entity(entity).despawn();
                }
            }
            for (entity, chunk) in query_active.iter() {
                if chunk.index == *index {
                    commands.entity(entity).despawn();
                }
            }
        }
    }

    /** Create the bind groups if they are not already created. */
    pub fn create_bind_groups(
        handler: Res<MCComputeHandlerGPU>, mut buffers: ResMut<RenderAssets<GpuBuffer>>,
        render_instance: Res<RenderInstance>, mut pipeline: ResMut<RenderAssets<GpuMCComputePipelineSpawn>>,
        mut registered_chunks: Query<&mut MCRegisteredChunk>,
        noise_parameters: Res<MCTerrainNoiseParameters>
    ) {
        // Get the compute pipeline
        let compute_pipeline = match pipeline.iter_mut().next() {
            Some((_, pipeline)) => pipeline,
            None => return
        };

        // Create the bind groups for the handler if they are not already created
        if compute_pipeline.desc_gpu_group.is_none() && handler.desc_gpu.is_some() {
            // Get the layouts
            let (desc_gpu_layout, noise_gpu_layout) = match (&compute_pipeline.desc_gpu_layout, &compute_pipeline.noise_gpu_layout) {
                (Some(desc_gpu_layout), Some(noise_gpu_layout)) => (desc_gpu_layout, noise_gpu_layout),
                _ => return
            };

            // Get the buffers
            let (desc_gpu, noise_parameters) = match (
                buffers.get(handler.desc_gpu.as_ref().unwrap()),
                buffers.get(handler.noise_parameters.as_ref().unwrap())
            ) {
                (Some(desc_gpu), Some(noise_parameters)) => (desc_gpu, noise_parameters),
                _ => return
            };

            // Create the bind groups
            let render_instance = render_instance.0.read().unwrap();
            let desc_gpu_bind_group = BindGroup::build(
                "marching-cubes-spawn-desc-gpu", &render_instance, &desc_gpu_layout.build(&render_instance),
                &vec![BindGroup::buffer(0, &desc_gpu.buffer)]);
            let noise_gpu_bind_group = BindGroup::build(
                "marching-cubes-spawn-noise-gpu", &render_instance, &noise_gpu_layout.build(&render_instance),
                &vec![BindGroup::buffer(0, &noise_parameters.buffer)]);

            // Update the handler
            compute_pipeline.desc_gpu_group = Some(desc_gpu_bind_group);
            compute_pipeline.noise_gpu_group = Some(noise_gpu_bind_group);
        }

        // Create the bind groups for the chunks if they are not already created
        let render_instance = render_instance.0.read().unwrap();
        for mut chunk in registered_chunks.iter_mut() {
            if chunk.points_gpu_group.is_none() {
                // Get the layout
                let points_layout = match &compute_pipeline.points_gpu_layout {
                    Some(points_layout) => points_layout,
                    _ => continue
                };

                // Get the buffer
                let points = match buffers.get(&chunk.points_gpu) {
                    Some(points) => points,
                    _ => continue
                };

                // Create the bind group
                let points_bind_group = BindGroup::build(
                    "marching-cubes-points-gpu", &render_instance, &points_layout.build(&render_instance),
                    &vec![BindGroup::buffer(0, &points.buffer)]);

                // Update the chunk
                chunk.points_gpu_group = Some(points_bind_group);
            }
        }

        // Update the noise buffer if there is at least one chunk
        if registered_chunks.iter().count() > 0 {
            if let Some(noise_buffer) = buffers.get_mut(handler.noise_parameters.as_ref().unwrap()) {
                noise_buffer.buffer.write(&render_instance,bytemuck::cast_slice(&[*noise_parameters]), 0);
            }
        }
    }

    /**
     * Generate the tasks for creating the points of each chunk.
     */
    pub fn compute(
        (query, mut commands): (Query<(Entity, &MCRegisteredChunk)>, Commands),
        (chunks_list, handler): (Res<MCChunksListRender>, Res<MCComputeHandlerGPU>),
        mut buffers: ResMut<RenderAssets<GpuBuffer>>,
        render_instance: Res<RenderInstance>,
        (pipeline, pipeline_manager): (
            Res<RenderAssets<GpuMCComputePipelineSpawn>>, Res<PipelineManager>
        )
    ) {
        // Get the compute pipeline
        let compute_pipeline = match pipeline.iter().next() {
            Some((_, pipeline)) => pipeline,
            None => return
        };

        // Check if the handler is ready
        let (desc_buffer_gpu, desc_buffer_group, noise_buffer_group) = match (
            &handler.desc_gpu, &compute_pipeline.desc_gpu_group, &compute_pipeline.noise_gpu_group
        ) {
            (Some(desc_buffer_gpu), Some(desc_buffer_group), Some(noise_buffer_group)) => (
                desc_buffer_gpu, desc_buffer_group, noise_buffer_group
            ),
            _ => return
        };

        // Generate the chunks
        let mut process_count = 0;
        for (entity, chunk) in query.iter() {
            process_count += 1;
            if process_count >= MC_MAX_CHUNKS_PROCESS_PER_FRAME {
                break;
            }
            let desc = chunks_list.chunks.get(&chunk.index).unwrap();

            // Update the description buffer
            trace!("Running the compute shader to compute the points for the chunk {:?}.", chunk.index);
            let desc_buff = GpuMCDescription {
                translation: [desc.translation.x, desc.translation.y, desc.translation.z, 0.0],
                chunk_length: [desc.length.x, desc.length.y, desc.length.z, 0.0],
                chunk_sub_count: [desc.sub_count.x, desc.sub_count.y, desc.sub_count.z, 0],
                triangles_counter: 0,
                iso_level: desc.iso_level,
                padding: [0, 0]
            };
            let render_instance = render_instance.0.read().unwrap();
            buffers.get_mut(desc_buffer_gpu).unwrap().buffer.write(&render_instance, bytemuck::cast_slice(&[desc_buff]), 0);

            // Create the compute pass
            let mut generated = false;
            let mut command_buffer = CommandBuffer::new(&render_instance, "marching-cubes-spawn");
            {
                let mut compute_pass = command_buffer.create_compute_pass("marching-cubes-spawn");

                // Set the pipeline
                if let (
                    CachedPipelineStatus::OkCompute(pipeline),
                    Some(points_group)
                ) = (
                    pipeline_manager.get_pipeline(compute_pipeline.cached_pipeline_index),
                    &chunk.points_gpu_group
                ) {
                    if compute_pass.set_pipeline(pipeline).is_err() {
                        continue;
                    }

                    // Set the bind groups
                    compute_pass.set_bind_group(0, desc_buffer_group);
                    compute_pass.set_bind_group(1, points_group);
                    compute_pass.set_bind_group(2, noise_buffer_group);

                    // Dispatch the compute pass
                    const NUM_THREADS: i32 = 10;
                    let dispatch_count_x = (desc.sub_count.x as f32 / NUM_THREADS as f32).ceil() as u32;
                    let dispatch_count_y = (desc.sub_count.y as f32 / NUM_THREADS as f32).ceil() as u32;
                    let dispatch_count_z = (desc.sub_count.z as f32 / NUM_THREADS as f32).ceil() as u32;
                    trace!("Dispatching the compute pass for spawning the chunk points {:?} with marching cubes with {} threads and {:?} dispatches.", entity, NUM_THREADS, [dispatch_count_x, dispatch_count_y, dispatch_count_z]);
                    if let Err(e) = compute_pass.dispatch(dispatch_count_x, dispatch_count_y, dispatch_count_z) {
                        error!("Failed to dispatch the compute pass for spawning the chunk points {:?} with marching cubes: {:?}", entity, e);
                        continue;
                    }
                    generated = true;
                }
            }
            if !generated {
                continue;
            }
            trace!("Spawned the chunk points for the chunk {:?} with marching cubes.", desc.index);

            // Submit the command buffer
            command_buffer.submit(&render_instance);

            // Spawn the chunk entity
            commands.entity(entity).despawn();
            commands.spawn(MCLoadingChunk {
                index: desc.index,
                points_gpu: chunk.points_gpu.clone(),
                points_gpu_group: None
            });
        }
    }
}
