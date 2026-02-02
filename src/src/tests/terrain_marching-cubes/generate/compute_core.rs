use bevy::prelude::*;
use wde_render::{assets::{GpuBuffer, RenderAssets}, core::RenderInstance, pipelines::{CachedPipelineStatus, PipelineManager}};
use wde_wgpu::{bind_group::BindGroup, command_buffer::CommandBuffer};

use crate::terrain::{mc_chunk::{MCChunksListRender, MCLoadingChunk, MCPendingChunk}, mc_compute_main::{GpuMCDescription, MCComputeHandlerGPU}, MC_MAX_CHUNKS_PROCESS_PER_FRAME, MC_MAX_TRIANGLES};

use super::compute_pipeline::GpuMCComputePipelineGenerate;

pub struct MCComputeCorePoints;
impl MCComputeCorePoints {
    /** Create the bind groups if they are not already created. */
    pub fn create_bind_groups(
        handler: Res<MCComputeHandlerGPU>, buffers: Res<RenderAssets<GpuBuffer>>,
        render_instance: Res<RenderInstance>, mut pipeline: ResMut<RenderAssets<GpuMCComputePipelineGenerate>>,
        mut loading_chunks: Query<&mut MCLoadingChunk>
    ) {
        // Get the compute pipeline
        let compute_pipeline = match pipeline.iter_mut().next() {
            Some((_, pipeline)) => pipeline,
            None => return
        };

        // Create the bind groups for the handler if they are not already created
        if compute_pipeline.desc_gpu_group.is_none() && handler.desc_gpu.is_some() {
            // Get the layouts
            let (desc_gpu_layout, triangles_layout) = match (
                &compute_pipeline.desc_gpu_layout,
                &compute_pipeline.triangles_gpu_layout
            ) {
                (Some(desc_gpu_layout), Some(triangles_layout)) => (
                    desc_gpu_layout, triangles_layout
                ),
                _ => return
            };

            // Get the buffers
            let (desc_gpu, triangles) = match (
                buffers.get(handler.desc_gpu.as_ref().unwrap()),
                buffers.get(handler.triangles_gpu.as_ref().unwrap()),
            ) {
                (Some(desc_gpu), Some(triangles)) => (
                    desc_gpu, triangles
                ),
                _ => return
            };

            // Create the bind groups
            let render_instance = render_instance.0.read().unwrap();
            let desc_gpu_bind_group = BindGroup::build(
                "marching-cubes-generate-desc-gpu", &render_instance, &desc_gpu_layout.build(&render_instance),
                &vec![BindGroup::buffer(0, &desc_gpu.buffer)]);
            let triangles_bind_group = BindGroup::build(
                "marching-cubes-generate-triangles-gpu", &render_instance, &triangles_layout.build(&render_instance),
                &vec![BindGroup::buffer(0, &triangles.buffer)]);

            // Update the handler
            compute_pipeline.desc_gpu_group = Some(desc_gpu_bind_group);
            compute_pipeline.triangles_gpu_group = Some(triangles_bind_group);
        }

        // Create the bind groups for the chunks if they are not already created
        let render_instance = render_instance.0.read().unwrap();
        for mut chunk in loading_chunks.iter_mut() {
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
    }
    
    /** Run the compute pass to generate the chunks. */
    pub fn compute(
        (query, mut commands): (Query<(Entity, &MCLoadingChunk)>, Commands),
        (chunks_list, handler): (Res<MCChunksListRender>, Res<MCComputeHandlerGPU>),
        mut buffers: ResMut<RenderAssets<GpuBuffer>>,
        render_instance: Res<RenderInstance>,
        (pipeline, pipeline_manager): (
            Res<RenderAssets<GpuMCComputePipelineGenerate>>, Res<PipelineManager>
        )
    ) {
        // Get the compute pipeline
        let compute_pipeline = match pipeline.iter().next() {
            Some((_, pipeline)) => pipeline,
            None => return
        };

        // Check if the handler is ready
        let (desc_buffer_cpu, desc_buffer_gpu, desc_buffer_group, triangles_cpu, triangles_gpu, triangles_group) = match (
            &handler.desc_cpu, &handler.desc_gpu, &compute_pipeline.desc_gpu_group, &handler.triangles_cpu, &handler.triangles_gpu, &compute_pipeline.triangles_gpu_group
        ) {
            (Some(desc_buffer_cpu), Some(desc_buffer_gpu), Some(desc_buffer_group), Some(triangles_cpu), Some(triangles_gpu), Some(triangles_group)) => (
                desc_buffer_cpu, desc_buffer_gpu, desc_buffer_group, triangles_cpu, triangles_gpu, triangles_group
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
            trace!("Running the compute shader to generate the triangles for the chunk {:?}.", chunk.index);
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
            let mut command_buffer = CommandBuffer::new(&render_instance, "marching-cubes-generate");
            {
                let mut compute_pass = command_buffer.create_compute_pass("marching-cubes-generate");

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
                    compute_pass.set_bind_group(2, triangles_group);

                    // Dispatch the compute pass
                    const NUM_THREADS: i32 = 10;
                    let dispatch_count_x = (desc.sub_count.x as f32 / NUM_THREADS as f32).ceil() as u32;
                    let dispatch_count_y = (desc.sub_count.y as f32 / NUM_THREADS as f32).ceil() as u32;
                    let dispatch_count_z = (desc.sub_count.z as f32 / NUM_THREADS as f32).ceil() as u32;
                    trace!("Dispatching the compute pass for generating chunk triangles {:?} with marching cubes with {} threads and {:?} dispatches.", entity, NUM_THREADS, [dispatch_count_x, dispatch_count_y, dispatch_count_z]);
                    if let Err(e) = compute_pass.dispatch(dispatch_count_x, dispatch_count_y, dispatch_count_z) {
                        error!("Failed to dispatch the compute pass for generating chunk triangles {:?} with marching cubes: {:?}", entity, e);
                        continue;
                    }
                    generated = true;
                }
            }
            if !generated {
                continue;
            }

            // Submit the command buffer
            command_buffer.submit(&render_instance);

            // Read the indices counter
            let mut triangles_counter = 0;
            let mut c_sub_count = [0, 0, 0];
            let cpu_buff = &buffers.get(desc_buffer_cpu).unwrap().buffer;
            cpu_buff.copy_from_buffer(&render_instance, &buffers.get(desc_buffer_gpu).unwrap().buffer);
            cpu_buff.map_read(&render_instance, |data| {
                let desc = bytemuck::from_bytes::<GpuMCDescription>(&data);
                triangles_counter = desc.triangles_counter;
                c_sub_count = [desc.chunk_sub_count[0] as usize, desc.chunk_sub_count[1] as usize, desc.chunk_sub_count[2] as usize];
            });

            // Warn if the indices counter is too high
            if triangles_counter > MC_MAX_TRIANGLES {
                error!("In the marching cubes algorithm, there is too much triangles overflowing the triangles buffer. The counter is {} while the maximum is {}.", triangles_counter, MC_MAX_TRIANGLES);
                continue;
            }

            // Read and process the triangles
            let mut raw_triangles: Vec<f32> = Vec::new();
            let triangles_buff = &buffers.get(triangles_cpu).unwrap().buffer;
            triangles_buff.copy_from_buffer(&render_instance, &buffers.get(triangles_gpu).unwrap().buffer);
            triangles_buff.map_read(&render_instance, |data| {
                let triangles: &[f32] = bytemuck::cast_slice(&data);
                raw_triangles.extend_from_slice(&triangles[..triangles_counter as usize * 12]);
            });

            // Spawn the chunk entity
            trace!("Spawning the chunk entity {:?} with {} triangles.", entity, triangles_counter);
            commands.entity(entity).despawn();
            commands.spawn(MCPendingChunk {
                index: chunk.index,
                raw_triangles,
                triangles_counter,
                points_gpu: chunk.points_gpu.clone()
            });
        }
    }
}
