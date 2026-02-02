use wde_logger::{prelude::*, tracing::span};

use bevy::prelude::*;
use wde_camera::features::CameraFeatureRender;
use wde_renderer::prelude::*;

use crate::{assets::PbrMaterialAsset, logic::{batches::Batches, ssbo::PbrSsbo, textures::PbrDeferredTextures}, prelude::{DirtyTransforms, PbrModelRegistry}};

use super::{GpuPbrGBufferRenderPipeline};


#[derive(Resource, Default)]
pub struct PbrGBufferRenderPass;
impl RenderPass for PbrGBufferRenderPass {
    fn extract(&self, main_world: &mut World, render_world: &mut World) {
        // Get the ssbo
        let ssbo_bf = {
            let _span = span!(Level::DEBUG, "get_pbr_ssbo_buffer");
            let buffers = render_world.get_resource::<RenderAssets<GpuBuffer>>().unwrap();
            match render_world.get_resource::<PbrSsbo>() {
                Some(buffer) => match buffers.get(&buffer.buffer_staging) {
                    Some(buffer) => buffer,
                    None => return
                },
                None => return
            }
        };

        // Get the ssbo instance-to-transform buffer
        let ssbo_instance_to_transform_buffer = {
            let _span = span!(Level::DEBUG, "get_instance_to_transform_ssbo_buffer");
            let buffers = render_world.get_resource::<RenderAssets<GpuBuffer>>().unwrap();
            match render_world.get_resource::<PbrSsbo>() {
                Some(buffer) => match buffers.get(&buffer.instance_to_transform_buffer) {
                    Some(buffer) => buffer,
                    None => return
                },
                None => return
            }
        };

        {
            let _span = span!(Level::DEBUG, "update_pbr_ssbo_buffer");
            
            // Get the list of dirty transforms
            let dirty_transforms = match main_world.get_resource::<DirtyTransforms>() {
                Some(dirty_transforms) => &dirty_transforms.0,
                None => return
            };
            if dirty_transforms.is_empty() {
                return;
            }

            // Update the dirty transforms in the ssbo buffer
            let render_instance = render_world.get_resource::<RenderInstance>().unwrap();
            let render_instance = render_instance.0.read().unwrap();
            let model_registry = main_world.get_resource::<PbrModelRegistry>().unwrap();
            ssbo_bf.buffer.map_write(&render_instance, |mut view| {
                let data = view.as_mut_ptr() as *mut TransformUniform;

                for (uuid, transform) in dirty_transforms.iter() {
                    // Get the transform ID
                    let transform_id = match model_registry.model_uuid_to_transform_id.get(uuid) {
                        Some(tid) => *tid,
                        None => {
                            warn!("No transform ID found for PbrModelElementUuid {}.", uuid);
                            0
                        }
                    };

                    // Write the transform uniform
                    let transform = TransformUniform::new(transform);
                    unsafe {
                        *data.add(transform_id as usize) = transform;
                    }
                }
            });
        }

        // Fill the ssbo instance-to-transform
        {
            let _span = span!(Level::DEBUG, "fill_instance_to_transform_ssbo");
            let batches = match render_world.get_resource::<Batches>() {
                Some(batches) => batches,
                None => return
            };
            
            // Write the instance-to-transform data
            let render_instance = render_world.get_resource::<RenderInstance>().unwrap();
            let render_instance = render_instance.0.read().unwrap();
            ssbo_instance_to_transform_buffer.buffer.map_write(&render_instance, |mut view| {
                let data = view.as_mut_ptr() as *mut u32;
                for (instance_index, transform_index) in batches.transform_ids.iter().enumerate() {
                    unsafe {
                        *data.add(instance_index) = *transform_index;
                    }
                }
            });
        }

        // Update the ssbo from the cpu buffer
        {
            let _span = span!(Level::DEBUG, "copy_pbr_ssbo_to_gpu");
            let render_instance = render_world.get_resource::<RenderInstance>().unwrap();
            let render_instance = render_instance.0.read().unwrap();
            let buffers = render_world.get_resource::<RenderAssets<GpuBuffer>>().unwrap();

            let ssbo_gpu = buffers.get(&render_world.get_resource::<PbrSsbo>().unwrap().buffer_gpu).unwrap();
            ssbo_gpu.buffer.copy_from_buffer(&render_instance, &ssbo_bf.buffer);
            let instance_to_transform_gpu = buffers.get(&render_world.get_resource::<PbrSsbo>().unwrap().instance_to_transform_buffer_gpu).unwrap();
            instance_to_transform_gpu.buffer.copy_from_buffer(&render_instance, &ssbo_instance_to_transform_buffer.buffer);
        }

        // Clear the dirty transforms
        {
            if let Some(mut dirty_transforms) = main_world.get_resource_mut::<DirtyTransforms>() {
                dirty_transforms.0.clear();
            }
        }
    }
    
    fn render(&self, render_world: &mut World) {
        // Get the render instance and swapchain frame
        let render_instance = render_world.get_resource::<RenderInstance>().unwrap();
        let render_instance = render_instance.0.read().unwrap();

        // Check if depth texture is ready
        let textures = render_world.get_resource::<RenderAssets<GpuTexture>>().unwrap();
        let depth_texture = match textures.get(&render_world.get_resource::<DepthTextureMSAA>().unwrap().texture) {
            Some(tex) => if render_instance.surface_config.as_ref().unwrap().width == tex.texture.size.0
                && render_instance.surface_config.as_ref().unwrap().height == tex.texture.size.1 {
                tex
            } else {
                return
            },
            None => return
        };

        // Check if pipeline is ready
        let gbuffer_pipeline = match render_world.get_resource::<RenderAssets<GpuPbrGBufferRenderPipeline>>() {
            Some(pipeline) => match pipeline.iter().next() {
                Some((_, pipeline)) => pipeline,
                None => return
            },
            None => return
        };

        // Check if the deferred textures are ready
        let deferred_textures = match render_world.get_resource::<PbrDeferredTextures>() {
            Some(textures) => textures,
            None => return
        };
        let (depth, depth_resolved, albedo, albedo_resolved, normal, normal_resolved) = match (
            textures.get(&deferred_textures.depth),
            textures.get(&deferred_textures.depth_resolved),
            textures.get(&deferred_textures.albedo),
            textures.get(&deferred_textures.albedo_resolved),
            textures.get(&deferred_textures.normal),
            textures.get(&deferred_textures.normal_resolved)
        ) {
            (Some(depth), Some(depth_resolved), Some(albedo), Some(albedo_resolved), Some(normal), Some(normal_resolved))
                => (depth, depth_resolved, albedo, albedo_resolved, normal, normal_resolved),
            _ => return
        };

        // Create the render pass
        let mut command_buffer = CommandBuffer::new(&render_instance, "gbuffer-pbr");
        {
            let mut render_pass = command_buffer.create_render_pass("gbuffer-pbr", |builder: &mut RenderPassBuilder| {
                builder.set_depth_texture(RenderPassDepth {
                    texture: Some(&depth_texture.texture.view),
                    ..Default::default()
                });
                builder.add_color_attachment(RenderPassColorAttachment {
                    texture: Some(&depth.texture.view),
                    resolve_target: Some(&depth_resolved.texture.view),
                    ..Default::default()
                });
                builder.add_color_attachment(RenderPassColorAttachment {
                    texture: Some(&albedo.texture.view),
                    resolve_target: Some(&albedo_resolved.texture.view),
                    ..Default::default()
                });
                builder.add_color_attachment(RenderPassColorAttachment {
                    texture: Some(&normal.texture.view),
                    resolve_target: Some(&normal_resolved.texture.view),
                    ..Default::default()
                });
            });

            // Get batches
            let batches = match render_world.get_resource::<Batches>() {
                Some(batches) => batches,
                None => return
            };

            // Get the materials and meshes
            let materials = render_world.get_resource::<RenderAssets<GpuMaterial<PbrMaterialAsset>>>().unwrap();
            let meshes = render_world.get_resource::<RenderAssets<GpuMesh>>().unwrap();

            // Render the mesh
            let pipeline_manager = render_world.get_resource::<PipelineManager>().unwrap();
            let camera_layout = render_world.get_resource::<CameraFeatureRender>().unwrap();
            let ssbo = render_world.get_resource::<PbrSsbo>().unwrap();
            if let (
                CachedPipelineStatus::OkRender(pipeline),
                Some(camera_bg),
                Some(ssbo_bind_group)
            ) = (
                pipeline_manager.get_pipeline(gbuffer_pipeline.cached_pipeline_index),
                &camera_layout.bind_group,
                &ssbo.bind_group
            ) {
                // Set the camera bind group
                render_pass.set_bind_group(0, camera_bg);

                // Set the pipeline
                if render_pass.set_pipeline(pipeline).is_ok() {
                    // Set the ssbo
                    render_pass.set_bind_group(1, ssbo_bind_group);

                    // For each set of mesh and material
                    let mut current_mesh_id = None;
                    let mut current_index_count = 0;
                    let mut current_material_id = None;
                    for batch in &batches.render_batches {
                        // Set the mesh
                        if current_mesh_id != Some(batch.mesh_id) {
                            let mesh = match meshes.get(batch.mesh_id) {
                                Some(mesh) => mesh,
                                None => continue
                            };
                            current_index_count = mesh.index_count;

                            // Set the mesh buffers
                            render_pass.set_vertex_buffer(0, &mesh.vertex_buffer);
                            render_pass.set_index_buffer(&mesh.index_buffer);
                            current_mesh_id = Some(batch.mesh_id);
                        }

                        // Set the material
                        if current_material_id != Some(batch.material_id) {
                            let material = match materials.get(batch.material_id) {
                                Some(material) => material,
                                None => continue
                            };

                            // Set the material bind group
                            render_pass.set_bind_group(2, &material.bind_group);
                            current_material_id = Some(batch.material_id);
                        }

                        // Draw the mesh
                        if let Err(e) = render_pass.draw_indexed(
                            0..current_index_count,
                            batch.first_instance..(batch.first_instance + batch.instance_count)
                        ) {
                            error!("Failed to draw: {:?}.", e);
                        }
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
