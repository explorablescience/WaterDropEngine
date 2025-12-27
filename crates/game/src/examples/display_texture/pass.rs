use bevy::prelude::*;
use wde_render::{assets::{GpuMaterial, GpuMesh, MeshAsset, ModelBoundingBox, RenderAssets}, core::SwapchainFrame, passes::render_graph::RenderPass, pipelines::{CachedPipelineStatus, PipelineManager}};
use wde_wgpu::{command_buffer::{RenderPassBuilder, RenderPassColorAttachment, WCommandBuffer}, instance::WRenderInstance, vertex::WVertex};

use crate::examples::display_texture::{material::{DisplayTextureMaterial, DisplayTextureMaterialAsset}, pipeline::GpuDisplayTexturePipeline};

#[derive(Resource, Default)]
pub struct RenderPassMesh {
    mesh: Option<Handle<MeshAsset>>
}
impl RenderPassMesh {
    // Creates the rendering mesh.
    pub fn init(assets_server: Res<AssetServer>, mut render_pass: ResMut<RenderPassMesh>) {
        // Create the 2d quad mesh
        let deferred_mesh: Handle<MeshAsset> = assets_server.add(MeshAsset {
            label: "texture-display-pass".to_string(),
            vertices: vec![
                WVertex { position: [-1.0, 1.0, 0.0], uv: [0.0, 1.0], normal: [0.0, 0.0, 0.0] },
                WVertex { position: [-1.0, -1.0, 0.0], uv: [0.0, 0.0], normal: [0.0, 0.0, 0.0] },
                WVertex { position: [1.0, -1.0, 0.0], uv: [1.0, 0.0], normal: [0.0, 0.0, 0.0] },
                WVertex { position: [1.0, 1.0, 0.0], uv: [1.0, 1.0], normal: [0.0, 0.0, 0.0] },
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            bounding_box: ModelBoundingBox {
                min: Vec3::new(-1.0, -1.0, 0.0),
                max: Vec3::new(1.0, 1.0, 0.0),
            },
        });
        render_pass.mesh = Some(deferred_mesh);
    }
}

#[derive(Resource, Default)]
pub struct GPUMaterialHandler {
    pub material: Option<Handle<DisplayTextureMaterialAsset>>,
}

#[derive(Resource, Default)]
pub struct DisplayTexturePass;
impl RenderPass for DisplayTexturePass {
    fn extract(&self, main_world: &mut World, render_world: &mut World) {
        // Extract the material handle
        let material_handle = match main_world.query_filtered::<&DisplayTextureMaterial, With<DisplayTextureMaterial>>().iter(main_world).next() {
            Some(material) => material.0.clone(),
            None => return
        };
        let mut material_handler = render_world.get_resource_mut::<GPUMaterialHandler>().unwrap();
        material_handler.material = Some(material_handle);

        // Extract the mesh for rendering
        let mesh_cpu = main_world.get_resource::<RenderPassMesh>().unwrap();
        let mut render_pass = render_world.get_resource_mut::<RenderPassMesh>().unwrap();
        render_pass.mesh = None;
        if let Some(ref mesh_cpu) = mesh_cpu.mesh {
            render_pass.mesh = Some(mesh_cpu.clone());
        }
    }

    fn render(&self, world: &mut World) {
        // Get the render instance and swapchain frame
        let render_instance = world.get_resource::<WRenderInstance>().unwrap();
        let render_instance = render_instance.data.read().unwrap();
        let swapchain_frame = world.get_resource::<SwapchainFrame>().unwrap().data.as_ref().unwrap();

        // Check if material is ready
        let material_handle = match &world.get_resource::<GPUMaterialHandler>().unwrap().material {
            Some(material) => material,
            None => return
        };

        // Check if mesh is ready
        let meshes = world.get_resource::<RenderAssets<GpuMesh>>().unwrap();
        let mesh = match &world.get_resource::<RenderPassMesh>().unwrap().mesh {
            Some(mesh) => match meshes.get(mesh) {
                Some(mesh) => mesh,
                None => return
            },
            None => return
        };

        // Check if pipeline is ready
        let pipeline_manager = world.get_resource::<PipelineManager>().unwrap();
        let pipeline = match world.get_resource::<RenderAssets<GpuDisplayTexturePipeline>>().unwrap().iter().next() {
            Some((_, pipeline)) => pipeline,
            None => return
        };
        
        // Render the texture
        let mut command_buffer = WCommandBuffer::new(&render_instance, "display-texture");
        {
            let mut render_pass = command_buffer.create_render_pass("display-texture", |builder: &mut RenderPassBuilder| {
                builder.add_color_attachment(RenderPassColorAttachment {
                    texture: Some(&swapchain_frame.view),
                    ..Default::default()
                });
            });

            let materials = world.get_resource::<RenderAssets<GpuMaterial<DisplayTextureMaterialAsset>>>().unwrap();
            if let (
                CachedPipelineStatus::OkRender(pipeline),
                Some(material)
            ) = (
                pipeline_manager.get_pipeline(pipeline.cached_pipeline_index),
                materials.get(material_handle)
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
