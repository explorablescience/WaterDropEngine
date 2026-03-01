use wde_logger::prelude::*;
use bevy::prelude::*;
use wde_renderer::prelude::*;
use wde_camera::features::CameraFeatureRender;

use crate::{
    render::terrain::{TILE_SIZE, TILE_SUBDIVISIONS, Terrain},
    render::passes::pipeline::GpuTerrainRenderPipeline,
    render::materials::TerrainMaterialArrays,
};

#[derive(Resource, Default)]
pub(crate) struct TerrainRenderPassMesh {
    pub deferred_mesh: Option<Handle<MeshAsset>>,
}
impl TerrainRenderPassMesh {
    // Creates the rendering mesh.
    pub fn init(assets_server: Res<AssetServer>, mut render_pass: ResMut<TerrainRenderPassMesh>) {
        let mut mesh = PlaneMesh::from("terrain_tile", TILE_SUBDIVISIONS, Vec3::Y);

        // Scale the plane to cover the entire terrain tile size
        for vertex in &mut mesh.vertices {
            vertex.position[0] *= TILE_SIZE;
            vertex.position[2] *= TILE_SIZE;
        }

        // Fix the UVs to cover the entire texture
        for vertex in &mut mesh.vertices {
            vertex.uv[0] = vertex.position[0] / TILE_SIZE + 0.5; // Map from [-TILE_SIZE/2, TILE_SIZE/2] to [0, 1]
            vertex.uv[1] = vertex.position[2] / TILE_SIZE + 0.5; // Map from [-TILE_SIZE/2, TILE_SIZE/2] to [0, 1]
        }

        render_pass.deferred_mesh = Some(assets_server.add(mesh));
    }
}

#[derive(Resource, Default)]
pub struct TerrainRenderPass;
impl RenderPass for TerrainRenderPass {
    fn extract(&self, main_world: &mut World, render_world: &mut World) {
        let _span = debug_span!("terrain_render_pass_extract").entered();

        // Extract the deferred mesh
        let mesh_cpu = main_world.get_resource::<TerrainRenderPassMesh>().unwrap();
        let mut render_pass = render_world
            .get_resource_mut::<TerrainRenderPassMesh>()
            .unwrap();
        render_pass.deferred_mesh = None;
        if let Some(ref mesh_cpu) = mesh_cpu.deferred_mesh {
            render_pass.deferred_mesh = Some(mesh_cpu.clone());
        }

        // Extract the material arrays bind group
        if let Some(material_arrays_cpu) = main_world.get_resource::<TerrainMaterialArrays>() {
            let mut material_arrays_render = render_world
                .get_resource_mut::<TerrainMaterialArrays>()
                .unwrap();
            
            // Only copy the bind group and layout, not the texture arrays themselves
            material_arrays_render.bind_group_layout = material_arrays_cpu.bind_group_layout.clone();
            material_arrays_render.bind_group = material_arrays_cpu.bind_group.clone();
        }
    }

    fn render(&self, world: &mut World) {
        let _span = debug_span!("terrain_render_pass_render").entered();

        // Get the tiles
        let terrain = match world.get_resource::<Terrain>() {
            Some(terrain) => terrain,
            None => return,
        };

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
            if let (CachedPipelineStatus::OkRender(pipeline), Some(camera_bind_group)) = (
                pipeline_manager.get_pipeline(pipeline.cached_pipeline_index),
                &world
                    .get_resource::<CameraFeatureRender>()
                    .unwrap()
                    .bind_group,
            ) {
                // Set the pipeline
                if render_pass.set_pipeline(pipeline).is_ok() {
                    for tile in &terrain.tiles {
                        // Get the mesh
                        render_pass.set_vertex_buffer(0, deferred_mesh.vertex_buffer.as_ref().unwrap());
                        render_pass.set_index_buffer(deferred_mesh.index_buffer.as_ref().unwrap());

                        // Set bind groups
                        render_pass.set_bind_group(0, camera_bind_group);
                        if let Some(bind_group) = &tile.bind_group {
                            render_pass.set_bind_group(1, bind_group);
                        } else {
                            continue;
                        }
                        // Set material arrays bind group (group 2)
                        render_pass.set_bind_group(2, material_arrays_bind_group);

                        // Draw the mesh
                        match render_pass.draw_indexed(0..deferred_mesh.index_count, 0..1) {
                            Ok(_) => {}
                            Err(e) => {
                                error!("Failed to draw: {:?}.", e);
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
