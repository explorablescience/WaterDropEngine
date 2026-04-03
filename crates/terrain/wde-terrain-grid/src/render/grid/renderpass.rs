use wde_logger::prelude::*;
use bevy::prelude::*;
use wde_renderer::prelude::*;
use wde_camera::prelude::*;
use wde_terrain::prelude::CHUNK_COUNT;

use crate::{editor::PlacementUI, render::grid::{buffers::TerrainGridBuffer, pipeline::GpuTerrainGridRenderPipeline}};

#[derive(Resource, Default)]
pub(crate) struct TerrainGridRenderPass {
    mesh: Option<Handle<MeshAsset>>,
    render_grid: bool
}
impl TerrainGridRenderPass {
    pub fn init(assets_server: Res<AssetServer>, mut render_pass: ResMut<TerrainGridRenderPass>) {
        // Create the 2d quad mesh
        let mesh: Handle<MeshAsset> = assets_server.add(MeshAsset {
            label: "terrain-grid-pass".to_string(),
            vertices: vec![
                Vertex { position: [-1.0, 1.0, 0.0], uv: [0.0, 1.0], ..Default::default() },
                Vertex { position: [-1.0, -1.0, 0.0], uv: [0.0, 0.0], ..Default::default() },
                Vertex { position: [1.0, -1.0, 0.0], uv: [1.0, 0.0], ..Default::default() },
                Vertex { position: [1.0, 1.0, 0.0], uv: [1.0, 1.0], ..Default::default() },
            ],
            indices: vec![0, 2, 1, 0, 3, 2],
            bounding_box: ModelBoundingBox {
                min: Vec3::new(-1.0, -1.0, 0.0),
                max: Vec3::new(1.0, 1.0, 0.0),
            },
            use_ssbo: false,
        });
        render_pass.mesh = Some(mesh);
    }

    pub fn extract(
        pass_main: ExtractWorld<Res<TerrainGridRenderPass>>,
        placement_ui: ExtractWorld<Res<PlacementUI>>,
        mut pass_render: ResMut<TerrainGridRenderPass>,
    ) {
        pass_render.mesh = pass_main.mesh.clone();
        pass_render.render_grid = placement_ui.enabled;
    }
}
impl RenderPassOld for TerrainGridRenderPass {
    fn render(&self, world: &mut World) {
        let terrain_grid_render_pass = world.get_resource::<TerrainGridRenderPass>().unwrap();
        if !terrain_grid_render_pass.render_grid {
            return;
        }
        let _span = debug_span!("terrain_grid_render_pass").entered();

        // Get buffers
        let grid_buffers = world.get_resource::<TerrainGridBuffer>().unwrap();
        if grid_buffers.bind_group.is_none() { return; }

        // Get the render instance and swapchain frame
        let render_instance = world.get_resource::<RenderInstance>().unwrap();
        let render_instance = render_instance.0.read().unwrap();
        let swapchain_frame = world.get_resource::<SwapchainFrame>().unwrap().data.as_ref().unwrap();

        // Check if depth texture is ready
        let textures = world.get_resource::<RenderAssets<GpuTexture>>().unwrap();
        let depth_texture = match textures
            .get(&world.get_resource::<DepthTexture>().unwrap().texture) {
                Some(tex) => {
                    if render_instance.surface_config.as_ref().unwrap().width == tex.texture.size.0
                        && render_instance.surface_config.as_ref().unwrap().height == tex.texture.size.1 { tex } else { return; }
                }
                None => return,
            };

        // Get the list of meshes
        let meshes = world.get_resource::<RenderAssets<GpuMesh>>().unwrap();
        let mesh_handle = terrain_grid_render_pass.mesh.as_ref().unwrap();
        let mesh = match meshes.get(mesh_handle) {
            Some(mesh) => mesh,
            None => return,
        };

        // Check if pipeline is ready
        let pipeline_manager = world.get_resource::<PipelineManager>().unwrap();
        let pipeline = match world.get_resource::<RenderAssets<GpuTerrainGridRenderPipeline>>().unwrap().iter()
            .next() {
                Some((_, pipeline)) => pipeline,
                None => return,
            };

        // Create the render pass
        let mut command_buffer = CommandBuffer::new(&render_instance, "terrain-grid");
        {
            let mut render_pass =
                command_buffer.create_render_pass("terrain-grid", |builder: &mut RenderPassBuilder| {
                    builder.set_depth_texture(RenderPassDepth {
                        texture: Some(&depth_texture.texture.view),
                        ..Default::default()
                    });
                    builder.add_color_attachment(RenderPassColorAttachment {
                        texture: Some(&swapchain_frame.view),
                        ..Default::default()
                    });
                    Ok(())
                }).unwrap();

            // Render the meshes
            if let (
                CachedPipelineStatus::OkRender(pipeline),
                Some(camera_bind_group),
                Some(vertex_buffer),
                Some(index_buffer)
            ) = (
                pipeline_manager.get_pipeline(pipeline.cached_pipeline_index),
                &world.get_resource::<CameraFeatureRender>().unwrap().bind_group,
                mesh.vertex_buffer.as_ref(),
                mesh.index_buffer.as_ref()
            ) {
                // Set the pipeline
                if render_pass.set_pipeline(pipeline).is_ok() {
                    // Set the vertex buffer
                    render_pass.set_vertex_buffer(0, vertex_buffer);
                    render_pass.set_index_buffer(index_buffer);

                    // Set the bind groups
                    render_pass.set_bind_group(0, camera_bind_group);
                    render_pass.set_bind_group(1, grid_buffers.bind_group.as_ref().unwrap());

                    // Draw the mesh
                    match render_pass.draw_indexed(0..6, 0..CHUNK_COUNT * CHUNK_COUNT) {
                        Ok(_) => {}
                        Err(e) => {
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

    fn label(&self) -> &str {
        "Terrain Grid"
    }
}
