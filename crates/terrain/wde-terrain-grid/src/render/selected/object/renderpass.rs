use wde_logger::prelude::*;
use bevy::prelude::*;
use wde_renderer::prelude::*;
use wde_camera::prelude::*;

use crate::render::selected::object::pipeline::GpuSelectedObjectRenderPipeline;

#[derive(Resource, Default)]
pub struct SelectedObjectRenderPass;
impl RenderPass for SelectedObjectRenderPass {
    fn render(&self, world: &mut World) {
        let _span = debug_span!("terrain_render_pass_render").entered();

        // Get the render instance and swapchain frame
        let render_instance = world.get_resource::<RenderInstance>().unwrap();
        let render_instance = render_instance.0.read().unwrap();
        let swapchain_frame = world
            .get_resource::<SwapchainFrame>()
            .unwrap()
            .data
            .as_ref()
            .unwrap();

        // Check if pipeline is ready
        let pipeline_manager = world.get_resource::<PipelineManager>().unwrap();
        let pipeline = match world
            .get_resource::<RenderAssets<GpuSelectedObjectRenderPipeline>>()
            .unwrap()
            .iter()
            .next()
        {
            Some((_, pipeline)) => pipeline,
            None => return,
        };

        // Create the render pass
        let mut command_buffer = CommandBuffer::new(&render_instance, "terrain");
        {
            let mut render_pass =
                command_buffer.create_render_pass("terrain", |builder: &mut RenderPassBuilder| {
                    builder.add_color_attachment(RenderPassColorAttachment {
                        texture: Some(&swapchain_frame.view),
                        ..Default::default()
                    });
                });

            // Render the meshes
            if let (
                CachedPipelineStatus::OkRender(pipeline),
                Some(camera_bind_group)
            ) = (
                pipeline_manager.get_pipeline(pipeline.cached_pipeline_index),
                &world
                    .get_resource::<CameraFeatureRender>()
                    .unwrap()
                    .bind_group
            ) {
                // Set the pipeline
                if render_pass.set_pipeline(pipeline).is_ok() {
                    // Set the bind groups
                    render_pass.set_bind_group(0, camera_bind_group);

                    // // Draw the mesh
                    // match render_pass.draw_indexed(0..deferred_mesh.index_count, i as u32..i as u32 + 1) {
                    //     Ok(_) => {}
                    //     Err(e) => {
                    //         error!("Failed to draw: {:?}.", e);
                    //     }
                    // }
                } else {
                    error!("Failed to set pipeline.");
                }
            }
        }

        // Submit the command buffer
        command_buffer.submit(&render_instance);
    }

    fn label(&self) -> &str {
        "Terrain Grid Selected Object"
    }
}
