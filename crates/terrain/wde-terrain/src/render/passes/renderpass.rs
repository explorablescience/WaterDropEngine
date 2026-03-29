use wde_logger::prelude::*;
use bevy::prelude::*;
use wde_renderer::prelude::*;
use wde_camera::prelude::*;

use crate::{prelude::{TerrainExtractor, TerrainRenderer}, render::{dependencies::{materials::TerrainMaterialArrays, terrain_buffer::TerrainBuffer, terrain_mesh::TerrainRenderPassMesh}, passes::pipeline::GpuTerrainRenderPipeline, renderer_gpu::TerrainRendererGPU}};

#[derive(Clone, Default)]
struct RenderPassDesc {
    label: String,
    attachments_colors: Option<()>, // If None, use swapchain texture
    attachments_depth: Option<AssetId<Texture>>
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum StageCommands {
    Pipeline(Option<CachedPipelineIndex>),
    BindGroup(u32, Option<BindGroup>), // index, bind group
    Mesh(Option<AssetId<MeshAsset>>)
}
#[derive(Clone, Default)]
struct SubPassDesc {
    global: Vec<StageCommands>
}

#[derive(Resource, Default)]
pub(crate) struct TerrainRenderPass;
impl TerrainRenderPass {
    pub fn extract(
        main_terrain_extractor: ExtractWorld<Res<TerrainExtractor>>,
        mut render_terrain_extractor: ResMut<TerrainExtractor>,
        terrain_renderer_query: ExtractWorld<Query<&TerrainRenderer>>,
        mut gpu_terrain_renderer: ResMut<TerrainRendererGPU>
    ) {
        let _span = debug_span!("terrain_render_pass_extract").entered();

        // Extract the dirty terrain tiles
        TerrainRendererGPU::extract_dirty(
            &main_terrain_extractor,
            &mut render_terrain_extractor,
            *terrain_renderer_query,
            &mut gpu_terrain_renderer
        );
    }
}
impl RenderPass for TerrainRenderPass {
    fn render(&self, world: &mut World) {
        let pass_desc = RenderPassDesc {
            label: "terrain".to_string(),
            attachments_colors: None,
            attachments_depth: Some(world.get_resource::<DepthTexture>().unwrap().texture.id())
        };
        let sub_pass_desc = SubPassDesc {
            global: vec![
                StageCommands::Pipeline(Some(world.get_resource::<RenderAssets<GpuTerrainRenderPipeline>>().unwrap().iter().next().map(|(_, p)| p.0)).flatten()),
                StageCommands::Mesh(world.get_resource::<TerrainRenderPassMesh>().unwrap().deferred_mesh.as_ref().map(|mesh| mesh.id())),
                StageCommands::BindGroup(0, world.get_resource::<CameraFeatureRender>().unwrap().bind_group.clone()),
                StageCommands::BindGroup(1, world.get_resource::<TerrainMaterialArrays>().unwrap().bind_group.clone()),
                StageCommands::BindGroup(2, world.get_resource::<TerrainBuffer>().unwrap().bind_group.clone())
            ]
        };


        let _span = debug_span!("render-pass-{}", pass_desc.label).entered();

        // Query render instance
        let render_instance = world.get_resource::<RenderInstance>().unwrap();
        let render_instance = render_instance.0.read().unwrap();

        // Get generic render assets handlers
        let textures = world.get_resource::<RenderAssets<GpuTexture>>().unwrap();
        let meshes = world.get_resource::<RenderAssets<GpuMesh>>().unwrap();
        let pipeline_manager = world.get_resource::<PipelineManager>().unwrap();

        // Handle pass
        let mut command_buffer = CommandBuffer::new(&render_instance, &pass_desc.label);
        {
            let mut should_return = false;
            let mut render_pass =
                command_buffer.create_render_pass(&pass_desc.label, |builder: &mut RenderPassBuilder| {
                    // Set color attachments
                    if pass_desc.attachments_colors.is_none() {
                        let swapchain_frame = world.get_resource::<SwapchainFrame>().unwrap().data.as_ref().unwrap();
                        builder.add_color_attachment(RenderPassColorAttachment {
                            texture: Some(&swapchain_frame.view),
                            ..default()
                        });
                    } else { should_return = true; }

                    // Set depth attachments
                    if pass_desc.attachments_depth.is_some()
                        && let Some(depth_texture) = textures.get(&world.get_resource::<DepthTexture>().unwrap().texture)
                        && render_instance.surface_config.as_ref().unwrap().width == depth_texture.texture.size.0
                        && render_instance.surface_config.as_ref().unwrap().height == depth_texture.texture.size.1
                    {
                        builder.set_depth_texture(RenderPassDepth {
                            texture: Some(&depth_texture.texture.view),
                            ..default()
                        });
                    } else { should_return = true; }
                });
            if should_return { return; }


            // Issue global commands
            for stage_command in &sub_pass_desc.global {
                match stage_command {
                    // Set pipeline
                    StageCommands::Pipeline(pipeline) => {
                        if let Some(pipeline) = pipeline
                            && let CachedPipelineStatus::OkRender(pipeline) = pipeline_manager.get_pipeline(*pipeline)
                        {
                            if let Err(e) = render_pass.set_pipeline(pipeline) {
                                error!("Failed to set pipeline: {:?}.", e);
                                return;
                            }
                        } else { return }
                    },
                    // Set bind groups at given index
                    StageCommands::BindGroup(index, bind_group) => {
                        if let Some(bind_group) = bind_group {
                            render_pass.set_bind_group(*index, bind_group);
                        } else { return }
                    },
                    StageCommands::Mesh(mesh) => {
                        if let Some(mesh) = mesh
                            && let Some(mesh) = meshes.get(*mesh)
                        {
                            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.as_ref().unwrap());
                            render_pass.set_index_buffer(mesh.index_buffer.as_ref().unwrap());
                        } else { return }
                    }
                }
            }

            // Render other commands
            if let Some(terrain) = &world.get_resource::<TerrainRendererGPU>() && terrain.ready {
                for (i, tile) in terrain.tiles.iter().enumerate() {
                    if let Some(bind_group) = &tile.render_bind_group {
                        // Set bind groups
                        render_pass.set_bind_group(3, bind_group);

                        // Draw the mesh
                        let deferred_mesh = world.get_resource::<TerrainRenderPassMesh>().unwrap().deferred_mesh.as_ref().unwrap();
                        let meshes = world.get_resource::<RenderAssets<GpuMesh>>().unwrap();
                        let mesh = meshes.get(deferred_mesh).unwrap();
                        match render_pass.draw_indexed(0..mesh.index_count, i as u32..i as u32 + 1) {
                            Ok(_) => {}
                            Err(e) => {
                                error!("Failed to draw: {:?}.", e);
                            }
                        }
                    }
                }
            }
        }
        command_buffer.submit(&render_instance);
    }

    fn name(&self) -> &str {
        "Terrain Ground"
    }
}
