use bevy::prelude::*;
use wde_render::{assets::{GpuMaterial, GpuMesh, Mesh, MeshAsset, RenderAssets}, core::SwapchainFrame, passes::render_graph::RenderPass, pipelines::{CachedPipelineStatus, PipelineManager}};
use wde_wgpu::{command_buffer::{RenderPassBuilder, RenderPassColorAttachment, WCommandBuffer}, instance::WRenderInstance};

use crate::examples::display_texture::{material::{DisplayTextureMaterial, DisplayTextureMaterialAsset}, pipeline::GpuDisplayTexturePipeline};

#[derive(Resource, Default)]
pub struct RenderPassEntity {
    pub mesh: Handle<MeshAsset>,
    pub material: Handle<DisplayTextureMaterialAsset>,
}

#[derive(Resource, Default)]
pub struct DisplayTexturePass;
impl RenderPass for DisplayTexturePass {
    fn extract(&self, main_world: &mut World, render_world: &mut World) {
        // Extract the mesh and material handles
        let entity = match main_world.query::<(Entity, &DisplayTextureMaterial, &Mesh)>().iter(main_world).next() {
            Some((entity, material, mesh)) => (entity, material.0.clone(), mesh.0.clone()),
            None => return
        };
        let mut render_pass_entity = render_world.get_resource_mut::<RenderPassEntity>().unwrap();
        render_pass_entity.material = entity.1;
        render_pass_entity.mesh = entity.2;
    }

    fn render(&self, world: &mut World) {
        // Get the render instance and swapchain frame
        let render_instance = world.get_resource::<WRenderInstance>().unwrap();
        let render_instance = render_instance.data.read().unwrap();
        let swapchain_frame = world.get_resource::<SwapchainFrame>().unwrap().data.as_ref().unwrap();

        // Check if pipeline is ready
        let pipeline_manager = world.get_resource::<PipelineManager>().unwrap();
        let pipeline = match world.get_resource::<RenderAssets<GpuDisplayTexturePipeline>>().unwrap().iter().next() {
            Some((_, pipeline)) => pipeline,
            None => return
        };

        // Get the entity to render
        let entity = world.get_resource::<RenderPassEntity>().unwrap();
        
        // Render the texture
        let mut command_buffer = WCommandBuffer::new(&render_instance, "display-texture");
        {
            let mut render_pass = command_buffer.create_render_pass("display-texture", |builder: &mut RenderPassBuilder| {
                builder.add_color_attachment(RenderPassColorAttachment {
                    texture: Some(&swapchain_frame.view),
                    ..Default::default()
                });
            });

            let meshes = world.get_resource::<RenderAssets<GpuMesh>>().unwrap();
            let materials = world.get_resource::<RenderAssets<GpuMaterial<DisplayTextureMaterialAsset>>>().unwrap();
            if let (
                CachedPipelineStatus::OkRender(pipeline),
                Some(material),
                Some(mesh)
            ) = (
                pipeline_manager.get_pipeline(pipeline.cached_pipeline_index),
                materials.get(&entity.material),
                meshes.get(&entity.mesh)
            ) {
                // Set the pipeline
                if render_pass.set_pipeline(pipeline).is_ok() {
                    // Get the mesh
                    render_pass.set_vertex_buffer(0, &mesh.vertex_buffer);
                    render_pass.set_index_buffer(&mesh.index_buffer);

                    // Set bind group
                    render_pass.set_bind_group(0, &material.bind_group);

                    // Draw the mesh
                    match render_pass.draw_indexed(0..mesh.index_count, 0..1) {
                        Ok(_) => {},
                        Err(e) => {
                            error!("Failed to draw: {:?}.", e);
                        }
                    };
                } else {
                    error!("Failed to set pipeline.");
                }
            }
        }

        // Submit the command buffer
        command_buffer.submit(&render_instance);
    }
}
