use wde_logger::prelude::*;
use bevy::prelude::*;

use crate::{passes::{depth_blit_pipeline::GpuDepthBlitRenderPipeline, depth_msaa::DepthMSAATextureLayout}, prelude::*};

#[derive(Resource, Default)]
pub(crate) struct DepthBlitRenderPass {
    mesh: Option<Handle<MeshAsset>>,
}
impl DepthBlitRenderPass {
    pub fn init(assets_server: Res<AssetServer>, mut render_pass: ResMut<DepthBlitRenderPass>) {
        // Create the 2d quad mesh
        let mesh: Handle<MeshAsset> = assets_server.add(MeshAsset {
            label: "depth-blit-pass".to_string(),
            vertices: vec![
                Vertex { position: [-1.0, 1.0, 0.0], uv: [0.0, 1.0], ..Default::default() },
                Vertex { position: [-1.0, -1.0, 0.0], uv: [0.0, 0.0], ..Default::default() },
                Vertex { position: [1.0, -1.0, 0.0], uv: [1.0, 0.0], ..Default::default() },
                Vertex { position: [1.0, 1.0, 0.0], uv: [1.0, 1.0], ..Default::default() },
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            bounding_box: ModelBoundingBox {
                min: Vec3::new(-1.0, -1.0, 0.0),
                max: Vec3::new(1.0, 1.0, 0.0),
            },
            use_ssbo: false,
        });
        render_pass.mesh = Some(mesh);
    }

    pub fn extract(
        pass_main: ExtractWorld<Res<DepthBlitRenderPass>>,
        mut pass_render: ResMut<DepthBlitRenderPass>,
    ) {
        pass_render.mesh = pass_main.mesh.clone();
    }
}
impl RenderPassOld for DepthBlitRenderPass {
    fn render(&self, render_world: &mut World) {
        // Get the render instance and swapchain frame
        let render_instance = render_world.get_resource::<RenderInstance>().unwrap();
        let render_instance = render_instance.0.read().unwrap();

        // Check if depth textures is ready
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

        // Get the MSAA layout
        let textures_layout = render_world.get_resource::<DepthMSAATextureLayout>().unwrap();
        if textures_layout.bind_group.is_none() || textures_layout.layout.is_none() {
            return;
        }

        // Get the depth blit pipeline
        let pass = match render_world.get_resource::<DepthBlitRenderPass>() {
            Some(pass) => pass,
            None => return
        };
        if pass.mesh.is_none() { // Nothing to render
            return;
        }

        // Get the buffers
        let meshes = render_world.get_resource::<RenderAssets<GpuMesh>>().unwrap();
        let mesh_handle = pass.mesh.as_ref().unwrap();
        let gpu_mesh = match meshes.get(mesh_handle) {
            Some(mesh) => mesh,
            None => return
        };

        // Check if pipeline is ready
        let depth_blit_pipeline = match render_world.get_resource::<RenderAssets<GpuDepthBlitRenderPipeline>>().unwrap().iter().next() {
            Some((_, pipeline)) => pipeline,
            None => return
        };

        // Create the render pass
        let mut command_buffer = CommandBuffer::new(&render_instance, "depth_blit");
        {
            let mut render_pass = command_buffer.create_render_pass("depth_blit", |builder: &mut RenderPassBuilder| {
                builder.set_depth_texture(RenderPassDepth {
                    texture: Some(&depth_texture.texture.view),
                    load: LoadOp::Load,
                    ..Default::default()
                });
                Ok(())
            }).unwrap();

            // Render the mesh
            let pipeline_manager = render_world.get_resource::<PipelineManager>().unwrap();
            if let CachedPipelineStatus::OkRender(pipeline) = pipeline_manager.get_pipeline(depth_blit_pipeline.cached_pipeline_index)  {
                // Set the pipeline
                if render_pass.set_pipeline(pipeline).is_ok() {
                    // Set the bind groups
                    render_pass.set_bind_group(0, textures_layout.bind_group.as_ref().unwrap());

                    // Set the mesh buffers
                    render_pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.as_ref().unwrap());
                    render_pass.set_index_buffer(gpu_mesh.index_buffer.as_ref().unwrap());

                    // Draw the mesh
                    match render_pass.draw_indexed(0..gpu_mesh.index_count, 0..1) {
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

    fn label(&self) -> &str {
        "Depth Blit"
    }
}
