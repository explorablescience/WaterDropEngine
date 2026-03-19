use bevy::prelude::*;
use wde::prelude::*;

use crate::display_texture::material::*;
use crate::display_texture::pipeline::*;

#[derive(Resource, Default)]
pub struct RenderPassEntity {
    pub mesh: Handle<MeshAsset>,
    pub material: Handle<DisplayTextureMaterialAsset>,
}

#[derive(Resource, Default)]
pub struct DisplayTexturePass;
impl DisplayTexturePass {
    pub fn extract(
        query: ExtractWorld<Query<(&DisplayTextureMaterial, &Mesh)>>,
        mut render_pass_entity: ResMut<RenderPassEntity>,
    ) {
        if let Some((material, mesh)) = query.iter().next() {
            render_pass_entity.material = material.0.clone();
            render_pass_entity.mesh = mesh.0.clone();
        }
    }
}
impl RenderPass for DisplayTexturePass {
    fn render(&self, world: &mut World) {
        // Get the render instance and swapchain frame
        let render_instance = world.get_resource::<RenderInstance>().unwrap();
        let render_instance = render_instance.0.read().unwrap();
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
        let mut command_buffer = CommandBuffer::new(&render_instance, "display-texture");
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
                    if let Some(vertex_buffer) = &mesh.vertex_buffer {
                        render_pass.set_vertex_buffer(0, vertex_buffer);
                    }
                    if let Some(index_buffer) = &mesh.index_buffer {
                        render_pass.set_index_buffer(index_buffer);
                    }

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
