use std::collections::HashMap;

use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::manager::{
    CHUNK_COUNT, CHUNK_RENDER_SUBDIVISIONS, ChunkPos, SPLAT_MAP_COUNT, Terrain, TerrainDirtyTile
};

/// Represents a single terrain tile, containing its position and the associated heightmap, normal map, and splat maps.
#[derive(Default, Clone)]
pub struct TerrainRenderTile {
    /// The position of the tile in world space (x, z)
    pub position: ChunkPos,
    /// The heightmap and splat maps for this tile
    pub heightmap: Handle<Texture>,
    pub splatmaps: Vec<Handle<Texture>>
}

/// Holds the tiles used for rendering. Note that all tiles are not necessarily rendered.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct TerrainRenderer {
    /// Pointer from tile position (x, z) to the corresponding tile datas
    pub pos_to_tile: HashMap<ChunkPos, usize>,
    /// A list of terrain render tiles that make up the entire terrain.
    #[reflect(ignore)]
    pub tiles: Vec<TerrainRenderTile>,
    // List of tile maps that are dirty and need to be re-processed
    #[reflect(ignore)]
    pub dirty: Vec<Option<TerrainDirtyTile>>
}
impl TerrainRenderer {
    /// Initializes the terrain renderer by creating the heightmap and splat map textures for each tile, and setting up the mapping from tile positions to their corresponding data.
    ///
    /// # Arguments
    /// * `asset_server` - The Bevy asset server used to create the texture assets for the heightmaps and splat maps.
    ///
    /// # Returns
    /// A `TerrainRenderer` component containing the initialized terrain render tiles with their respective heightmap and splat map textures, as well as the mapping from tile positions to their data.
    pub fn new(asset_server: &AssetServer) -> Self {
        let usages = TextureUsages::COPY_SRC
            | TextureUsages::COPY_DST
            | TextureUsages::TEXTURE_BINDING
            | TextureUsages::STORAGE_BINDING;
        let mut pos_to_tile = HashMap::new();
        let mut tiles = Vec::new();
        for i in 0..CHUNK_COUNT {
            for j in 0..CHUNK_COUNT {
                // Create the empty heightmap and splatmap textures for the tile
                let heightmap = asset_server.add(Texture {
                    label: format!("heightmap_{}_{}", i, j),
                    size: (CHUNK_RENDER_SUBDIVISIONS, CHUNK_RENDER_SUBDIVISIONS),
                    format: TextureFormat::R8Unorm,
                    usages,
                    ..Default::default()
                });
                let mut splatmaps = Vec::new();
                for k in 0..SPLAT_MAP_COUNT / 4 {
                    splatmaps.push(asset_server.add(Texture {
                        label: format!("splatmap_{}_{}-{}", i, j, k),
                        size: (CHUNK_RENDER_SUBDIVISIONS, CHUNK_RENDER_SUBDIVISIONS),
                        format: TextureFormat::Rgba8Unorm,
                        usages,
                        ..Default::default()
                    }));
                }

                // Calculate the world position of the tile (centered around the origin)
                let px = i as i32 - (CHUNK_COUNT as i32 / 2);
                let pz = j as i32 - (CHUNK_COUNT as i32 / 2);
                let position = IVec2::new(px, pz);

                // Create the tile and add it to the list
                tiles.push(TerrainRenderTile {
                    position,
                    heightmap,
                    splatmaps
                });
                pos_to_tile.insert(position, tiles.len() - 1);
            }
        }
        TerrainRenderer {
            dirty: Vec::new(),
            tiles,
            pos_to_tile
        }
    }

    /// Extracts the dirty tiles from the main terrain
    pub fn extract_dirty(
        mut renderer: Query<&mut TerrainRenderer>,
        mut terrain: Query<&mut Terrain>
    ) {
        let mut terrain_renderer = match renderer.iter_mut().next() {
            Some(terrain) => terrain,
            None => return
        };
        let mut terrain = match terrain.iter_mut().next() {
            Some(terrain) => terrain,
            None => return
        };

        // Clear the dirty tiles list before processing
        terrain_renderer.dirty.clear();

        // Extract the dirty tiles from the main terrain and move them to the renderer resource
        for dirty_tile in &terrain.dirty_render {
            terrain_renderer.dirty.push(dirty_tile.clone());
        }

        // Clear the dirty tiles list after processing
        terrain.dirty_render.clear();
    }
}
