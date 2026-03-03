use wde_logger::prelude::*;
use bevy::prelude::*;
use wde_renderer::prelude::*;
use wde_camera::features::CameraFeatureRender;

use crate::render::{dependencies::{materials::TerrainMaterialArrays, terrain_buffer::TerrainBuffer, terrain_mesh::TerrainRenderPassMesh}, passes::{tiles_extractor::GpuTerrainTiles, pipeline::GpuTerrainRenderPipeline}};

#[derive(Resource, Default)]
pub struct TerrainRenderPass;
impl RenderPass for TerrainRenderPass {
    fn extract(&self, main_world: &mut World, render_world: &mut World) {
        let _span = debug_span!("terrain_render_pass_extract").entered();

        // Extract the dirty terrain tiles
        GpuTerrainTiles::extract_tiles(main_world, render_world);
    }

    fn render(&self, world: &mut World) {
        let _span = debug_span!("terrain_render_pass_render").entered();

        // Get material arrays bind group
        let material_arrays_bind_group = match world.get_resource::<TerrainMaterialArrays>() {
            Some(arrays) => match &arrays.bind_group {
                Some(bg) => bg,
                None => {
                    // Material arrays not ready yet
                    return;
                }
            },
            None => return,
        };

        // Get the render instance and swapchain frame
        let render_instance = world.get_resource::<RenderInstance>().unwrap();
        let render_instance = render_instance.0.read().unwrap();
        let swapchain_frame = world
            .get_resource::<SwapchainFrame>()
            .unwrap()
            .data
            .as_ref()
            .unwrap();

        // Check if deferred mesh is ready
        let meshes = world.get_resource::<RenderAssets<GpuMesh>>().unwrap();
        let deferred_mesh = match &world
            .get_resource::<TerrainRenderPassMesh>()
            .unwrap()
            .deferred_mesh
        {
            Some(mesh) => match meshes.get(mesh) {
                Some(mesh) => mesh,
                None => return,
            },
            None => return,
        };

        // Check if depth texture is ready
        let textures = world.get_resource::<RenderAssets<GpuTexture>>().unwrap();
        let depth_texture = match textures
            .get(&world.get_resource::<DepthTexture>().unwrap().texture)
        {
            Some(tex) => {
                if render_instance.surface_config.as_ref().unwrap().width == tex.texture.size.0
                    && render_instance.surface_config.as_ref().unwrap().height == tex.texture.size.1
                {
                    tex
                } else {
                    return;
                }
            }
            None => return,
        };

        // Check if pipeline is ready
        let pipeline_manager = world.get_resource::<PipelineManager>().unwrap();
        let pipeline = match world
            .get_resource::<RenderAssets<GpuTerrainRenderPipeline>>()
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
                    builder.set_depth_texture(RenderPassDepth {
                        texture: Some(&depth_texture.texture.view),
                        ..Default::default()
                    });
                    builder.add_color_attachment(RenderPassColorAttachment {
                        texture: Some(&swapchain_frame.view),
                        ..Default::default()
                    });
                });

            // Render the meshes
            if let (
                CachedPipelineStatus::OkRender(pipeline),
                Some(camera_bind_group),
                Some(terrain_description_bind_group),
                Some(terrain)
            ) = (
                pipeline_manager.get_pipeline(pipeline.cached_pipeline_index),
                &world
                    .get_resource::<CameraFeatureRender>()
                    .unwrap()
                    .bind_group,
                &world.get_resource::<TerrainBuffer>().unwrap().bind_group,
                &world.get_resource::<GpuTerrainTiles>()
            ) {
                // Set the pipeline
                if render_pass.set_pipeline(pipeline).is_ok() {
                    // Get the mesh
                    render_pass.set_vertex_buffer(0, deferred_mesh.vertex_buffer.as_ref().unwrap());
                    render_pass.set_index_buffer(deferred_mesh.index_buffer.as_ref().unwrap());

                    // Set the bind groups
                    render_pass.set_bind_group(0, camera_bind_group);
                    render_pass.set_bind_group(1, material_arrays_bind_group);
                    render_pass.set_bind_group(2, terrain_description_bind_group);

                    for (i, tile) in terrain.ready_tiles.iter().enumerate() {
                        if let Some(bind_group) = &tile.bind_group {
                            // Set bind groups
                            render_pass.set_bind_group(3, bind_group);

                            // Draw the mesh
                            match render_pass.draw_indexed(0..deferred_mesh.index_count, i as u32..i as u32 + 1) {
                                Ok(_) => {}
                                Err(e) => {
                                    error!("Failed to draw: {:?}.", e);
                                }
                            }
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
