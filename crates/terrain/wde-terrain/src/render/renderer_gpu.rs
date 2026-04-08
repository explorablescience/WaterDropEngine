use std::collections::HashMap;

use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::{
    manager::{ChunkPos, SPLAT_MAP_COUNT, TerrainDirtyTile},
    prelude::TerrainExtractor,
    render::{extractor, renderer::TerrainRenderer}
};

#[derive(Asset, Clone, TypePath, Default)]
pub struct TerrainTileBgRender {
    tile_position: ChunkPos,
    heightmap: Handle<Texture>,
    splatmaps: Vec<Handle<Texture>>
}
impl RenderBinding for TerrainTileBgRender {
    fn describe(&self, builder: &mut RenderBindingBuilder) {
        builder.add_texture_view(0, Some(self.heightmap.clone()));
        builder.add_texture_sampler(1, Some(self.heightmap.clone()));
        for i in 0..SPLAT_MAP_COUNT / 4 {
            builder.add_texture_view(2 + i * 2, Some(self.splatmaps[i as usize].clone()));
            builder.add_texture_sampler(3 + i * 2, Some(self.splatmaps[i as usize].clone()));
        }
    }

    fn label(&self) -> &str {
        Box::leak(
            format!(
                "terrain-tile-{}-{}-render",
                self.tile_position.x, self.tile_position.y
            )
            .into_boxed_str()
        )
    }
}

#[derive(Asset, Clone, TypePath, Default)]
pub struct TerrainTileBgCompute {
    tile_position: ChunkPos,
    heightmap: Handle<Texture>,
    splatmaps: Vec<Handle<Texture>>
}
impl RenderBinding for TerrainTileBgCompute {
    fn describe(&self, builder: &mut RenderBindingBuilder) {
        builder.add_storage_texture(0, Some(self.heightmap.clone()));
        for i in 0..SPLAT_MAP_COUNT / 4 {
            builder.add_storage_texture(1 + i, Some(self.splatmaps[i as usize].clone()));
        }
    }

    fn label(&self) -> &str {
        Box::leak(
            format!(
                "terrain-tile-{}-{}-compute",
                self.tile_position.x, self.tile_position.y
            )
            .into_boxed_str()
        )
    }
}

/// Same as `TerrainRenderTile`, but with the texture handles replaced by their corresponding asset IDs. Only present on the GPU side.
#[derive(Default, Clone)]
pub struct TerrainRenderTileGPU {
    /// The position of the tile in world space (x, z)
    pub position: ChunkPos,

    /// The heightmap and splat maps for this tile
    pub heightmap: Handle<Texture>,
    pub splatmaps: Vec<Handle<Texture>>,

    // The associated render and compute bind groups
    pub render_bind_group: Option<Handle<TerrainTileBgRender>>,
    pub compute_bind_group: Option<Handle<TerrainTileBgCompute>>
}

/// Holds the tiles used for rendering. Note that all tiles are not necessarily rendered.
#[derive(Resource, Default)]
pub struct TerrainRendererGPU {
    /// True if the renderer has been initialized (i.e., the bind group of each tiles have been created and is ready for rendering)
    pub ready: bool,
    /// Mapping of the tile position (x, z) to its tile index in the tiles vector
    pub pos_to_tile: HashMap<ChunkPos, usize>,
    /// A list of terrain render tiles that make up the entire terrain.
    pub tiles: Vec<TerrainRenderTileGPU>,
    // List of tile maps that are dirty and need to be re-processed
    pub dirty: Vec<Option<TerrainDirtyTile>>
}
impl TerrainRendererGPU {
    // Extract the dirty tiles from the main world and move them to the GPU renderer resource.
    pub fn extract_dirty(
        main_terrain_extractor: &TerrainExtractor,
        render_terrain_extractor: &mut TerrainExtractor,
        terrain_renderer_query: Query<&TerrainRenderer>,
        gpu_terrain_renderer: &mut TerrainRendererGPU
    ) {
        // Run extractor if needed
        extractor::extract_dirty(main_terrain_extractor, render_terrain_extractor);

        // Get the terrain renderer resource and the GPU terrain tiles resource
        let terrain_renderer = match terrain_renderer_query.iter().next() {
            Some(terrain) => terrain,
            None => return
        };

        // If it is the first time the terrain is ready, extract the tiles data from the main world and move them to the GPU renderer resource
        if gpu_terrain_renderer.tiles.is_empty() {
            gpu_terrain_renderer.tiles = terrain_renderer
                .tiles
                .iter()
                .map(|tile| TerrainRenderTileGPU {
                    position: tile.position,
                    heightmap: tile.heightmap.clone(),
                    splatmaps: tile.splatmaps.to_vec(),
                    render_bind_group: None,
                    compute_bind_group: None
                })
                .collect();
            gpu_terrain_renderer.pos_to_tile = terrain_renderer.pos_to_tile.clone();
        }

        // Extract the dirty tiles
        for i in 0..terrain_renderer.dirty.len() {
            if let Some(dirty_tile) = &terrain_renderer.dirty[i] {
                gpu_terrain_renderer.dirty.push(Some(dirty_tile.clone()));
            }
        }
    }

    // Upload the dirty tiles to the GPU by updating the corresponding textures with the new data, and clear the dirty list.
    pub fn upload_dirty(
        mut gpu_terrain_renderer: ResMut<TerrainRendererGPU>,
        textures: Res<RenderAssets<GpuTexture>>,
        render_instance: Res<RenderInstance>
    ) {
        let render_instance = render_instance.0.read().unwrap();

        // Process the dirty tiles by updating the corresponding textures with the new data
        for i in 0..gpu_terrain_renderer.dirty.len() {
            if let Some(dirty_tile) = gpu_terrain_renderer.dirty[i].take() {
                let (pos, map_type, splat_map_index, tile_data) = dirty_tile;
                let tile_index = match gpu_terrain_renderer.pos_to_tile.get(&pos) {
                    Some(index) => *index,
                    None => continue
                };
                let tile = &mut gpu_terrain_renderer.tiles[tile_index];
                match map_type {
                    0 => {
                        // Heightmap
                        let heightmap = match textures.get(&tile.heightmap) {
                            Some(texture) => texture,
                            None => continue
                        };
                        heightmap.texture.copy_from_buffer(
                            &render_instance,
                            heightmap.texture.format,
                            bytemuck::cast_slice(&tile_data)
                        );
                    }
                    1 => {
                        // Splatmap
                        let splatmap = match textures.get(&tile.splatmaps[splat_map_index as usize])
                        {
                            Some(texture) => texture,
                            None => continue
                        };
                        splatmap.texture.copy_from_buffer(
                            &render_instance,
                            splatmap.texture.format,
                            bytemuck::cast_slice(&tile_data)
                        );
                    }
                    _ => unreachable!()
                };
            }
        }
    }

    // Create the bind groups and layouts for tiles that are not ready
    pub fn prepare_bind_groups(
        asset_server: Res<AssetServer>,
        mut gpu_terrain_renderer: ResMut<TerrainRendererGPU>
    ) {
        // Check if all tiles gpu data is read
        if gpu_terrain_renderer.ready {
            return;
        }

        // Create the bind groups for the dirty tiles
        for tile in &mut gpu_terrain_renderer.tiles {
            // Check if already processed
            if tile.render_bind_group.is_some() {
                continue;
            }

            // Create the bind group layout for the render tile
            tile.render_bind_group = Some(asset_server.add(TerrainTileBgRender {
                tile_position: tile.position,
                heightmap: tile.heightmap.clone(),
                splatmaps: tile.splatmaps.to_vec()
            }));

            // Create the bind group layout for the compute tile
            tile.compute_bind_group = Some(asset_server.add(TerrainTileBgCompute {
                tile_position: tile.position,
                heightmap: tile.heightmap.clone(),
                splatmaps: tile.splatmaps.to_vec()
            }));
        }

        // Check if all tiles are ready
        gpu_terrain_renderer.ready = gpu_terrain_renderer
            .tiles
            .iter()
            .all(|tile| tile.render_bind_group.is_some());
    }
}
