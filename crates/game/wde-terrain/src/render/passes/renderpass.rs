use wde_logger::prelude::*;
use bevy::prelude::*;
use wde_renderer::prelude::*;
use wde_camera::features::CameraFeatureRender;

use crate::render::{materials::TerrainMaterialArrays, passes::pipeline::GpuTerrainRenderPipeline, renderer::{SPLAT_MAP_COUNT, TILE_SIZE, TILE_SUBDIVISIONS, TerrainRenderer}};

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

#[derive(Default, Clone)]
pub(crate) struct ExtractedTerrainTile {
    // The position of the tile in world space (x, z)
    pub position: Vec2,

    // The heightmap, normal map, and splat maps for this tile
    pub heightmap: AssetId<Texture>,
    pub normalmap: AssetId<Texture>,
    pub splatmaps: Vec<AssetId<Texture>>,

    // The bind group for the different maps
    pub bind_group_layout: Option<BindGroupLayout>,
    pub bind_group: Option<BindGroup>
}

#[derive(Resource, Default)]
pub struct TerrainRenderPass {
    // The position of the tiles that are ready to be rendered (i.e., have their textures loaded and bind groups created)
    pub ready_tile_positions: Vec<Vec2>,
    // The tiles that are ready to be rendered (i.e., have their textures loaded and bind groups created)
    pub ready_tiles: Vec<ExtractedTerrainTile>,
    // The tiles that were extracted from the main world but may not be ready for rendering yet (e.g., waiting for textures to load)
    pub extracted_tiles: Vec<ExtractedTerrainTile>,
}
impl TerrainRenderPass {
    // Prepare the bind groups and layouts of the newly extracted tiles, and move them to the ready list once they are ready.
    pub fn prepare_tiles(mut render_pass: ResMut<TerrainRenderPass>, mut textures: ResMut<RenderAssets<GpuTexture>>, render_instance: Res<RenderInstance>) {
        let render_instance = render_instance.0.read().unwrap();
        for tile in &mut render_pass.extracted_tiles {
            // Get the maps
            let (heightmap, normalmap, splatmaps) = match (
                textures.get(tile.heightmap),
                textures.get(tile.normalmap),
                tile.splatmaps.iter().map(|splatmap| textures.get(*splatmap)).collect::<Option<Vec<_>>>(),
            ) {
                (Some(heightmap), Some(normalmap), Some(splatmaps)) => (heightmap, normalmap, splatmaps),
                _ => continue,
            };

            // Create the bind group layout
            let ss = ShaderStages::FRAGMENT | ShaderStages::VERTEX;
            let bind_group_layout = BindGroupLayout::new(&format!("terrain-tile-{}-{}", tile.position.x, tile.position.y), |builder: &mut BindGroupLayoutBuilder| {
                builder.add_texture_view(   0, ss, false);
                builder.add_texture_sampler(1, ss);
                builder.add_texture_view(   2, ss, false);
                builder.add_texture_sampler(3, ss);
                for i in 0..SPLAT_MAP_COUNT / 4 {
                    builder.add_texture_view(   4 + i * 2, ss, false);
                    builder.add_texture_sampler(5 + i * 2, ss);
                }
            });

            // Build the layout
            let bind_group_layout_built = BindGroupLayout::build(&bind_group_layout, &render_instance);

            // Create the bind group
            let bind_group = BindGroupBuilder::build(&format!("terrain-tile-{}-{}", tile.position.x, tile.position.y), &render_instance, &bind_group_layout_built, &{
                let mut entries = vec![
                    BindGroupBuilder::texture_view(   0, &heightmap.texture),
                    BindGroupBuilder::texture_sampler(1, &heightmap.texture),
                    BindGroupBuilder::texture_view(   2, &normalmap.texture),
                    BindGroupBuilder::texture_sampler(3, &normalmap.texture),
                ];
                for i in 0..SPLAT_MAP_COUNT / 4 {
                    entries.push(BindGroupBuilder::texture_view(   4 + i * 2, &splatmaps[i as usize].texture));
                    entries.push(BindGroupBuilder::texture_sampler(5 + i * 2, &splatmaps[i as usize].texture));
                }
                entries
            });

            // Mark the textures as clean
            {
                textures.get_mut(tile.heightmap).unwrap().dirty = false;
                textures.get_mut(tile.normalmap).unwrap().dirty = false;
                for splatmap in &tile.splatmaps {
                    textures.get_mut(*splatmap).unwrap().dirty = false;
                }
            }

            // Insert the resources
            tile.bind_group_layout = Some(bind_group_layout);
            tile.bind_group = Some(bind_group);
        }

        // Move ready tiles to the ready list
        let mut i = 0;
        while i < render_pass.extracted_tiles.len() {
            if render_pass.extracted_tiles[i].bind_group.is_some() {
                let tile = render_pass.extracted_tiles.remove(i);
                render_pass.ready_tile_positions.push(tile.position);
                render_pass.ready_tiles.push(tile);
            } else {
                i += 1;
            }
        }
    }

    /// Check if any texture of any tile has been modified (e.g., file changed on disk). If so, remove the tile from the ready list and add it back to the extracted list to be re-prepared.
    pub fn check_dirty_tiles(mut render_pass: ResMut<TerrainRenderPass>, textures: Res<RenderAssets<GpuTexture>>) {
        let mut i = 0;
        while i < render_pass.ready_tiles.len() {
            let tile = &render_pass.ready_tiles[i];
            let mut dirty = false;
            if let Some(heightmap) = textures.get(tile.heightmap) && heightmap.dirty {
                dirty = true;
            }
            if let Some(normalmap) = textures.get(tile.normalmap) && normalmap.dirty {
                dirty = true;
            }
            for splatmap in &tile.splatmaps {
                if let Some(splatmap) = textures.get(*splatmap) && splatmap.dirty {
                    dirty = true;
                    break;
                }
            }
            if dirty {
                // Move the tile back to the extracted list
                let tile = render_pass.ready_tiles.remove(i);
                render_pass.extracted_tiles.push(tile);
            } else {
                i += 1;
            }
        }
    }
}
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

        // Extract the terrain tiles
        {
            let mut terrain = match main_world.query::<&mut TerrainRenderer>().iter_mut(main_world).next() {
                Some(terrain) => terrain,
                None => return,
            };
            let mut render_pass = render_world
                .get_resource_mut::<TerrainRenderPass>()
                .unwrap();
            for tile in &mut terrain.tiles {
                let position = tile.position;

                // Process new tiles
                if render_pass.ready_tile_positions.contains(&position) && !tile.dirty {
                    continue;
                }
                tile.dirty = false;
                render_pass.extracted_tiles.push(ExtractedTerrainTile {
                    position,
                    heightmap: tile.heightmap.id(),
                    normalmap: tile.normalmap.id(),
                    splatmaps: tile.splatmaps.iter().map(|splatmap| splatmap.id()).collect(),
                    bind_group_layout: None,
                    bind_group: None
                });
            }
        }
    }

    fn render(&self, world: &mut World) {
        let _span = debug_span!("terrain_render_pass_render").entered();

        // Get the tiles
        let terrain = match world.get_resource::<TerrainRenderPass>() {
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
                    for tile in &terrain.ready_tiles {
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
