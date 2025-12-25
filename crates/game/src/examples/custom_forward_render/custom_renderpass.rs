use bevy::prelude::*;
use wde_render::{assets::{GpuBuffer, GpuMaterial, GpuMesh, GpuTexture, Mesh, MeshAsset, RenderAssets}, components::TransformUniform, core::{extract_macros::ExtractWorld, SwapchainFrame}, features::CameraFeatureRender, pipelines::{CachedPipelineStatus, PipelineManager}, passes::depth::DepthTexture};
use wde_wgpu::{command_buffer::{RenderPassBuilder, RenderPassColorAttachment, RenderPassDepth, WCommandBuffer}, instance::WRenderInstance};

use super::{CustomMaterial, CustomMaterialAsset, CustomSsbo, GpuCustomRenderPipeline};

pub struct CustomRenderBatch {
    mesh: Handle<MeshAsset>,
    material: Handle<CustomMaterialAsset>,
    first: usize,
    count: usize,
    index_count: usize,
}

#[derive(Resource)]
pub struct CustomRenderPass {
    pub batches: Vec<CustomRenderBatch>,
}
impl CustomRenderPass {
    /// Create the batches with the correct mesh and material.
    pub fn create_batches(
        mut pass: ResMut<CustomRenderPass>, render_instance: Res<WRenderInstance<'static>>,
        entities: ExtractWorld<Query<(&Transform, &Mesh, &CustomMaterial)>>,
        meshes: Res<RenderAssets<GpuMesh>>, materials: Res<RenderAssets<GpuMaterial<CustomMaterialAsset>>>,
        buffers: Res<RenderAssets<GpuBuffer>>, ssbo: Res<CustomSsbo>
    ) {
        // Clear the batches of the previous frame
        pass.batches.clear();

        // Get the ssbo
        let ssbo_bf = match buffers.get(&ssbo.buffer) {
            Some(ssbo) => ssbo,
            None => return
        };
        
        // If no entities, return
        if entities.is_empty() {
            return
        }

        // Create the batches
        let render_instance = render_instance.data.read().unwrap();
        ssbo_bf.buffer.map_write(&render_instance, |mut view| {
            let mut first = 0;
            let mut count = 1;
            let mut last_mesh: Option<Handle<MeshAsset>> = None;
            let mut last_material: Option<Handle<CustomMaterialAsset>> = None;
            let data = view.as_mut_ptr() as *mut TransformUniform;

            for (transform, mesh, material) in entities.iter() {
                // Check if new element in same batch
                let last_mesh_ref = last_mesh.as_ref();
                let last_material_ref = last_material.as_ref();
                if last_mesh_ref.is_some() && last_material_ref.is_some() {
                    if mesh.0.id() == last_mesh_ref.unwrap().id() && material.0.id() == last_material_ref.unwrap().id() {
                        // Update the ssbo
                        let transform = TransformUniform::new(transform);
                        unsafe {
                            *data.add(first + count) = transform;
                        }

                        // Increment the count
                        count += 1;

                        continue;
                    } else {
                        // Push the batch
                        pass.batches.push(CustomRenderBatch {
                            mesh: last_mesh_ref.unwrap().clone(),
                            material: last_material_ref.unwrap().clone(),
                            first,
                            count,
                            index_count: match meshes.get(last_mesh_ref.unwrap()) {
                                Some(mesh) => mesh.index_count as usize,
                                None => 0
                            }
                        });

                        // Reset the batch
                        first += count;
                        count = 1;
                        last_mesh = None;
                        last_material = None;
                    }
                }

                // Update the last mesh and ssbo if loaded
                let mut updated_mesh = false;
                let mut updated_material = false;
                if meshes.get(&mesh.0).is_some() {
                    last_mesh = Some(mesh.0.clone());
                    updated_mesh = true;
                }
                if materials.get(&material.0).is_some() {
                    last_material = Some(material.0.clone());
                    updated_material = true;
                }
                if updated_mesh && updated_material {
                    // Update the ssbo
                    let transform = TransformUniform::new(transform);
                    unsafe {
                        *data.add(first) = transform;
                    }
                }
            }

            // Push the last batch
            if let (Some(last_mesh), Some(last_material)) = (last_mesh, last_material) {
                pass.batches.push(CustomRenderBatch {
                    mesh: last_mesh.clone(),
                    material: last_material.clone(),
                    first,
                    count,
                    index_count: match meshes.get(&last_mesh) {
                        Some(mesh) => mesh.index_count as usize,
                        None => 0
                    }
                });
            }
        });

        // Update the ssbo
        let ssbo_gpu = match buffers.get(&ssbo.buffer_gpu) {
            Some(ssbo) => ssbo,
            None => return
        };
        ssbo_gpu.buffer.copy_from_buffer(&render_instance, &ssbo_bf.buffer);
    }


    /// Render the different batches.
    pub fn render(
        (render_instance, swapchain_frame, pipeline_manager): (
            Res<WRenderInstance<'static>>, Res<SwapchainFrame>,  Res<PipelineManager>
        ),
        (camera_layout, ssbo) : (Res<CameraFeatureRender>, Res<CustomSsbo>),
        (meshes, textures, materials): (
            Res<RenderAssets<GpuMesh>>, Res<RenderAssets<GpuTexture>>, Res<RenderAssets<GpuMaterial<CustomMaterialAsset>>>
        ),
        (mesh_pipeline, render_mesh_pass, depth_texture): (
            Res<RenderAssets<GpuCustomRenderPipeline>>, Res<CustomRenderPass>, Res<DepthTexture>
        )
    ) {
        // Get the render instance and swapchain frame
        let render_instance = render_instance.data.read().unwrap();
        let swapchain_frame = swapchain_frame.data.as_ref().unwrap();

        // Check if depth texture is ready
        let depth_texture = match textures.get(&depth_texture.texture) {
            Some(tex) => if render_instance.surface_config.as_ref().unwrap().width == tex.texture.size.0
                && render_instance.surface_config.as_ref().unwrap().height == tex.texture.size.1 {
                tex
            } else {
                return
            },
            None => return
        };

        // Check if pipeline is ready
        let mesh_pipeline = match mesh_pipeline.iter().next() {
            Some((_, pipeline)) => pipeline,
            None => return
        };

        // Create the render pass
        let mut command_buffer = WCommandBuffer::new(&render_instance, "custom");
        {
            let mut render_pass = command_buffer.create_render_pass("custom",
            |builder: &mut RenderPassBuilder| {
                builder.set_depth_texture(RenderPassDepth {
                    texture: Some(&depth_texture.texture.view),
                    ..Default::default()
                });
                builder.add_color_attachment(RenderPassColorAttachment {
                    texture: Some(&swapchain_frame.view),
                    ..Default::default()
                });
            });

            // Render the mesh
            if let (
                CachedPipelineStatus::OkRender(pipeline),
                Some(camera_bg),
                Some(ssbo_bind_group)
            ) = (
                pipeline_manager.get_pipeline(mesh_pipeline.cached_pipeline_index),
                &camera_layout.bind_group,
                &ssbo.bind_group
            ) {
                // Set the camera bind group
                render_pass.set_bind_group(0, camera_bg);

                // Set the pipeline
                if render_pass.set_pipeline(pipeline).is_ok() {
                    // Set the ssbo
                    render_pass.set_bind_group(1, ssbo_bind_group);

                    let mut old_mesh_id = None;
                    let mut old_material_id = None;
                    for batch in render_mesh_pass.batches.iter() {
                        // Set the material
                        if old_material_id != Some(batch.material.id()) {
                            let material = match materials.get(&batch.material) {
                                Some(material) => material,
                                None => continue // Should not happen
                            };

                            // Set the material bind group
                            render_pass.set_bind_group(2, &material.bind_group);
                            old_material_id = Some(batch.material.id());
                        }

                        // Set the mesh
                        if old_mesh_id != Some(batch.mesh.id()) {
                            let mesh = match meshes.get(&batch.mesh) {
                                Some(mesh) => mesh,
                                None => continue // Should not happen
                            };

                            // Set the mesh buffers
                            render_pass.set_vertex_buffer(0, &mesh.vertex_buffer);
                            render_pass.set_index_buffer(&mesh.index_buffer);
                            old_mesh_id = Some(batch.mesh.id());
                        }

                        // Draw the mesh
                        let instance_indices = batch.first as u32..((batch.first + batch.count) as u32);
                        match render_pass.draw_indexed(0..batch.index_count as u32, instance_indices) {
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
