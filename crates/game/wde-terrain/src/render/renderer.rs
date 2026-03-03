use std::collections::HashMap;

use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::manager::{DirtyTile, SPLAT_MAP_COUNT, TERRAIN_TILES_COUNT, RENDER_TILE_SUBDIVISIONS, Terrain, TilePos};

/// Represents a single terrain tile, containing its position and the associated heightmap, normal map, and splat maps.
#[derive(Default, Clone)]
pub struct TerrainRenderTile {
    /// The position of the tile in world space (x, z)
    pub position: TilePos,
    /// The heightmap and splat maps for this tile
    pub heightmap: Handle<Buffer>,
    pub splatmaps: Vec<Handle<Buffer>>
}

/// Holds the tiles used for rendering. Note that all tiles are not necessarily rendered.
#[derive(Component)]
pub struct TerrainRenderer {
    /// Pointer from tile position (x, z) to the corresponding tile datas
    pub pos_to_tile: HashMap<TilePos, usize>,
    /// A list of terrain render tiles that make up the entire terrain.
    pub tiles: Vec<TerrainRenderTile>,
    // List of tile maps that are dirty and need to be re-processed
    pub dirty: Vec<Option<DirtyTile>>
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
        let usage = BufferUsage::STORAGE | BufferUsage::COPY_DST;
        let mut pos_to_tile = HashMap::new();
        let mut tiles = Vec::new();
        for i in 0..TERRAIN_TILES_COUNT {
            for j in 0..TERRAIN_TILES_COUNT {
                // Create the empty heightmap and splatmap textures for the tile
                let heightmap = asset_server.add(Buffer {
                    label: format!("heightmap_{}_{}", i, j),
                    size: (RENDER_TILE_SUBDIVISIONS * RENDER_TILE_SUBDIVISIONS) as usize * std::mem::size_of::<f32>(),
                    usage,
                    content: None
                });
                let mut splatmaps = Vec::new();
                for k in 0..SPLAT_MAP_COUNT / 4 {
                    splatmaps.push(asset_server.add(Buffer {
                        label: format!("splatmap_{}_{}-{}", i, j, k),
                        size: (RENDER_TILE_SUBDIVISIONS * RENDER_TILE_SUBDIVISIONS * 4) as usize * std::mem::size_of::<f32>(),
                        usage,
                        content: None
                    }));
                }

                // Calculate the world position of the tile (centered around the origin)
                let px = i as i32 - (TERRAIN_TILES_COUNT as i32 / 2);
                let pz = j as i32 - (TERRAIN_TILES_COUNT as i32 / 2);
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
        TerrainRenderer { dirty: Vec::new(), tiles, pos_to_tile }
    }

    /// Extracts the dirty tiles from the main terrain
    pub fn extract_dirty(mut renderer: Query<&mut TerrainRenderer>, mut terrain: Query<&mut Terrain>) {
        let mut terrain_renderer = match renderer.iter_mut().next() {
            Some(terrain) => terrain,
            None => return,
        };
        let mut terrain = match terrain.iter_mut().next() {
            Some(terrain) => terrain,
            None => return,
        };

        // Extract the dirty tiles from the main terrain and move them to the renderer resource
        for dirty_tile in &terrain.dirty_render {
            terrain_renderer.dirty.push(dirty_tile.clone());
        }

        // Clear the dirty tiles list after processing
        terrain.dirty_render.clear();
    }
}
