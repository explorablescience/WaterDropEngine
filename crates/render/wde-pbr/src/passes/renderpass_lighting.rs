use wde_logger::prelude::*;
use bevy::prelude::*;
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::{logic::{lights::LightsFeatureBuffer, textures::PbrDeferredTexturesLayout}, passes::pipeline_lighting::GpuPbrLightingRenderPipeline};


#[derive(Resource, Default)]
pub(crate) struct PbrLightingRenderPassMesh {
    pub deferred_mesh: Option<Handle<MeshAsset>>
}
impl PbrLightingRenderPassMesh {
    // Creates the rendering mesh.
    pub fn init(assets_server: Res<AssetServer>, mut render_pass: ResMut<PbrLightingRenderPassMesh>) {
        // Create the 2d quad mesh
        let deferred_mesh: Handle<MeshAsset> = assets_server.add(MeshAsset {
            label: "deferred-lighting-pass".to_string(),
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
            use_ssbo: false
        });
        render_pass.deferred_mesh = Some(deferred_mesh);
    }

    pub fn extract(
        pass_main: ExtractWorld<Res<PbrLightingRenderPassMesh>>,
        mut pass_render: ResMut<PbrLightingRenderPassMesh>,
    ) {
        pass_render.deferred_mesh = None;
        if let Some(ref mesh_cpu) = pass_main.deferred_mesh {
            pass_render.deferred_mesh = Some(mesh_cpu.clone());
        }
    }
}

#[derive(Resource, Default)]
pub struct PbrLightingRenderPass;
impl RenderPass for PbrLightingRenderPass {
    fn render(&self, world: &mut World) {
        let _span = debug_span!("lighting_pbr_render_pass_render").entered();

        // Get the render instance and swapchain frame
        let render_instance = world.get_resource::<RenderInstance>().unwrap();
        let render_instance = render_instance.0.read().unwrap();
        let swapchain_frame = world.get_resource::<SwapchainFrame>().unwrap().data.as_ref().unwrap();

        // Check if mesh is ready
        let meshes = world.get_resource::<RenderAssets<GpuMesh>>().unwrap();
        let deferred_mesh = match &world.get_resource::<PbrLightingRenderPassMesh>().unwrap().deferred_mesh {
            Some(mesh) => match meshes.get(mesh) {
                Some(mesh) => mesh,
                None => return
            },
            None => return
        };

        // Check if pipeline is ready
        let pipeline_manager = world.get_resource::<PipelineManager>().unwrap();
        let lighting_pipeline = match world.get_resource::<RenderAssets<GpuPbrLightingRenderPipeline>>().unwrap().iter().next() {
            Some((_, pipeline)) => pipeline,
            None => return
        };

        // Create the render pass
        let mut command_buffer = CommandBuffer::new(&render_instance, "lighting-pbr");
        {
            let mut render_pass = command_buffer.create_render_pass("lighting-pbr", |builder: &mut RenderPassBuilder| {
                builder.add_color_attachment(RenderPassColorAttachment {
                    texture: Some(&swapchain_frame.view),
                    ..Default::default()
                });
            });

            // Render the mesh
            if let (
                CachedPipelineStatus::OkRender(pipeline),
                Some(camera_bind_group),
                Some(deferred_bind_group_resolved),
                Some(lights_bind_group)
            ) = (
                pipeline_manager.get_pipeline(lighting_pipeline.cached_pipeline_index),
                &world.get_resource::<CameraFeatureRender>().unwrap().bind_group,
                &world.get_resource::<PbrDeferredTexturesLayout>().unwrap().deferred_bind_group_resolved,
                &world.get_resource::<LightsFeatureBuffer>().unwrap().bind_group
            ) {
                // Set the pipeline
                if render_pass.set_pipeline(pipeline).is_ok() {
                    // Get the mesh
                    render_pass.set_vertex_buffer(0, deferred_mesh.vertex_buffer.as_ref().unwrap());
                    render_pass.set_index_buffer(deferred_mesh.index_buffer.as_ref().unwrap());

                    // Set bind groups
                    render_pass.set_bind_group(0, camera_bind_group);
                    render_pass.set_bind_group(1, deferred_bind_group_resolved);
                    render_pass.set_bind_group(2, lights_bind_group);
                    
                    // Draw the mesh
                    match render_pass.draw_indexed(0..deferred_mesh.index_count, 0..1) {
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
