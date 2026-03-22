use std::sync::{Arc, RwLock};

use egui_wgpu::{Renderer, RendererOptions, ScreenDescriptor};
use wde_renderer::{assets::SWAPCHAIN_FORMAT, prelude::*};
use bevy::prelude::*;

use crate::egui::egui_context::EguiFrameData;

/// Plugin to add the egui render pass
pub(crate) struct EguiRenderPassPlugin;
impl Plugin for EguiRenderPassPlugin {
    fn build(&self, app: &mut App) {
        // Register the extract system
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Extract, EguiRenderPass::extract);

        // Add the render pass to the render graph
        let mut render_graph = app.get_sub_app_mut(RenderApp).unwrap()
            .world_mut().get_resource_mut::<RenderGraph>().unwrap();
        render_graph.add_pass::<EguiRenderPass>(1001); // Add after main render pass
    }

    fn finish(&self, app: &mut App) {
        let renderpass = EguiRenderPass::new(app.get_sub_app_mut(RenderApp).unwrap().world_mut());
        app.get_sub_app_mut(RenderApp).unwrap()
            .insert_resource(renderpass);
    }
}

/// Resource to store egui render pass and renderer
#[derive(Resource, Default)]
pub(crate) struct EguiRenderPass {
    pub renderer: Option<Arc<RwLock<Renderer>>>,
}
impl EguiRenderPass {
    pub fn new(world: &mut World) -> Self {
        // Get render instance
        let render_instance = world.get_resource::<RenderInstance>().unwrap();
        let render_instance = render_instance.0.read().unwrap();

        // Create egui renderer
        let egui_rpass = Renderer::new(&render_instance.device, SWAPCHAIN_FORMAT, RendererOptions::default());
        EguiRenderPass {
            renderer: Some(Arc::new(RwLock::new(egui_rpass))),
        }
    }

    fn extract(
        frame_data_main: ExtractWorld<Res<EguiFrameData>>,
        mut frame_data_render: ResMut<EguiFrameData>,
    ) {
        frame_data_render.paint_jobs = frame_data_main.paint_jobs.clone();
        frame_data_render.textures_delta = frame_data_main.textures_delta.clone();
    }
}
impl RenderPass for EguiRenderPass {
    fn render(&self, world: &mut World) {
        // Get the render instance and swapchain frame
        let render_instance = world.get_resource::<RenderInstance>().unwrap();
        let render_instance = render_instance.0.read().unwrap();
        let swapchain_frame = world.get_resource::<SwapchainFrame>().unwrap().data.as_ref().unwrap();

        // Get renderer
        let egui_renderer = match world.get_resource::<EguiRenderPass>().unwrap().renderer.as_ref() {
            Some(renderer) => renderer,
            None => return,
        };

        // Get frame data
        let swapchain_descriptor = render_instance.surface_config.as_ref().unwrap();
        let screen_descriptor = ScreenDescriptor {
            pixels_per_point: 1.0,
            size_in_pixels: [swapchain_descriptor.width, swapchain_descriptor.height],
        };

        // Get paint jobs and textures delta
        let frame_data = world.get_resource::<EguiFrameData>().unwrap();
        let paint_jobs = match frame_data.paint_jobs.as_ref() {
            Some(jobs) => jobs,
            None => return,
        };
        let textures_delta = match frame_data.textures_delta.as_ref() {
            Some(delta) => delta,
            None => return,
        };

        // Create the render pass
        let mut command_buffer = CommandBuffer::new(&render_instance, "egui");
        {
            let render_pass = command_buffer.create_render_pass("egui", |builder: &mut RenderPassBuilder| {
                builder.add_color_attachment(RenderPassColorAttachment {
                    texture: Some(&swapchain_frame.view),
                    load: LoadOp::Load,
                    ..Default::default()
                });
            }).forget_lifetime();

            // Update egui textures
            for (id, image_delta) in &textures_delta.set {
                egui_renderer.write().unwrap().update_texture(&render_instance.device, &render_instance.queue, *id, image_delta);
            }
            for id in &textures_delta.free {
                egui_renderer.write().unwrap().free_texture(id);
            }
            
            // Draw egui
            egui_renderer.write().unwrap().update_buffers(&render_instance.device, &render_instance.queue, command_buffer.encoder(), paint_jobs, &screen_descriptor);
            egui_renderer.read().unwrap().render(&mut render_pass.into_inner(), paint_jobs, &screen_descriptor);
        }
        command_buffer.submit(&render_instance);
    }

    fn name(&self) -> &str {
        "Egui"
    }
}
