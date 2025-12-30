use bevy::prelude::*;
use wde_render::{assets::{GpuBuffer, GpuTexture, RenderAssets}, core::SwapchainFrame, features::{CameraFeatureRender, LightsFeatureBuffer}, passes::{depth::DepthTexture, render_graph::RenderPass}, pipelines::{CachedPipelineStatus, PipelineManager}};
use wde_wgpu::{command_buffer::{RenderPassBuilder, RenderPassColorAttachment, RenderPassDepth, CommandBuffer, LoadOp}, instance::RenderInstance};

use crate::terrain::mc_chunk::MCActiveChunk;

use super::render_pipeline::GpuMCRenderPipeline;

#[derive(Default)]
pub struct MCRenderPass;
impl RenderPass for MCRenderPass {
    fn render(&self, render_world: &mut World) {
        // Get the active chunks
        let mut active_chunks = render_world.query::<&MCActiveChunk>();
        if active_chunks.iter(render_world).count() == 0 {
            return;
        }

        // Get the render instance and swapchain frame
        let render_instance = render_world.get_resource::<RenderInstance<'static>>().unwrap();
        let render_instance = render_instance.data.read().unwrap();

        // Check if depth texture is ready
        let textures = render_world.get_resource::<RenderAssets<GpuTexture>>().unwrap();
        let depth_texture = match textures.get(&render_world.get_resource::<DepthTexture>().unwrap().texture) {
            Some(tex) => if render_instance.surface_config.as_ref().unwrap().width == tex.texture.size.0
                && render_instance.surface_config.as_ref().unwrap().height == tex.texture.size.1 {
                tex
            } else {
                return
            },
            None => return
        };

        // Check if pipeline is ready
        let mcbuffer_pipeline = match render_world.get_resource::<RenderAssets<GpuMCRenderPipeline>>().unwrap().iter().next() {
            Some((_, pipeline)) => pipeline,
            None => return
        };

        // Test if swapchain frame and depth texture have the same size
        let swapchain_frame = render_world.get_resource::<SwapchainFrame>().unwrap();
        let swapchain_frame = swapchain_frame.data.as_ref().unwrap();
        if swapchain_frame.texture.texture.size().width != depth_texture.texture.size.0 || swapchain_frame.texture.texture.size().height != depth_texture.texture.size.1 {
            warn!("Swapchain frame and depth texture have different sizes: {:?} vs {:?}.", swapchain_frame.texture.texture.size(), depth_texture.texture.size);
            return;
        }
        
        // Create the render pass
        let mut command_buffer = CommandBuffer::new(&render_instance, "marching-cubes");
        {
            let mut render_pass = command_buffer.create_render_pass("marching-cubes", |builder: &mut RenderPassBuilder| {
                builder.add_color_attachment(RenderPassColorAttachment {
                    texture: Some(&swapchain_frame.view),
                    load: LoadOp::Load,
                    ..Default::default()
                });
                builder.set_depth_texture(RenderPassDepth {
                    texture: Some(&depth_texture.texture.view),
                    load_operation: LoadOp::Load,
                    ..Default::default()
                });
            });

            // Render the mesh
            let pipeline_manager = render_world.get_resource::<PipelineManager>().unwrap();
            if let (
                CachedPipelineStatus::OkRender(pipeline),
                Some(camera_bg),
                Some(lights_bg)
            ) = (
                pipeline_manager.get_pipeline(mcbuffer_pipeline.cached_pipeline_index),
                &render_world.get_resource::<CameraFeatureRender>().unwrap().bind_group,
                &render_world.get_resource::<LightsFeatureBuffer>().unwrap().bind_group
            ) {
                // Set the camera bind group
                render_pass.set_bind_group(0, camera_bg);
                render_pass.set_bind_group(1, lights_bg);

                // Set the pipeline
                if render_pass.set_pipeline(pipeline).is_ok() {
                    let buffers = render_world.get_resource::<RenderAssets<GpuBuffer>>().unwrap();
                    for chunk in active_chunks.iter(render_world) {
                        // Get the vertex and index buffers
                        let (vertex_buffer, index_buffer) = match (
                            buffers.get(&chunk.vertices),
                            buffers.get(&chunk.indices)
                        ) {
                            (Some(vertex_buffer), Some(index_buffer)) => (vertex_buffer, index_buffer),
                            _ => continue
                        };
                        
                        // Set the mesh buffers
                        render_pass.set_vertex_buffer(0, &vertex_buffer.buffer);
                        render_pass.set_index_buffer(&index_buffer.buffer);

                        // Draw the mesh
                        match render_pass.draw_indexed(0..chunk.indices_counter, 0..1) {
                            Ok(_) => {},
                            Err(e) => {
                                error!("Failed to draw: {:?}.", e);
                            }
                        };
                    }
                } else {
                    error!("Failed to set pipeline.");
                }
            }
        }

        // Submit the command buffer
        command_buffer.submit(&render_instance);
    }
}
