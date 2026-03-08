use std::collections::HashMap;

use wde_logger::prelude::*;
use bevy::prelude::*;

use crate::utils::image_decoder::decode_png_as_channels;

/// The size of the terrain in terms of number of tiles (e.g., 4 means the terrain is made up of a 4x4 grid of tiles)
pub const TERRAIN_TILES_COUNT: usize = 8;
/// The size of each terrain tile in world units
pub const TILE_SIZE: [f32; 3] = [500.0, 1000.0, 500.0];

/// The number of splat maps per tile (must be a multiple of 4, as each splat map can store 4 channels for texture blending)
pub const SPLAT_MAP_COUNT: u32 = 4;
/// The number of subdivisions per tile (e.g., 16 means each tile is divided into a 16x16 grid of vertices)
/// This should match the width and height of the texture maps.
pub const RENDER_TILE_SUBDIVISIONS: u32 = 256;

/// The size of the heightmap subdivisions per tile (e.g., 16 means the heightmap is divided into a 16x16 grid of height samples)
pub const PHYSICS_TILE_SUBDIVISIONS: u32 = 256;

// The structure describing the 2d position of a given terrain tile.
pub type TilePos = IVec2;
// The raw data of a terrain tile, containing the heightmap and splat maps as byte arrays. This is the format used for storing as a texture.
pub type TileData = Vec<u8>;
// A dirty tile that needs to be re-processed, containing its position, the type of map that is dirty (0 for heightmap, 1 for splatmap), the index of the splat map if the map type is splatmap, and the new tile data.
pub type DirtyTile = (TilePos, u32, u32, TileData);

pub struct TerrainTile {
    pub pos: TilePos,
    pub heightmap: TileData,
    pub splatmaps: Vec<TileData>,
}

/// The main terrain resource that holds all the terrain tiles.
/// This is the source of truth for the terrain data, and is used by both the physics and rendering systems.
#[derive(Component)]
pub struct Terrain {
    /// Path to the terrain
    pub path: String,
    /// Pointer from tile position (x, z) to the corresponding tile datas
    pub pos_to_tile: HashMap<TilePos, usize>,

    /// List of tile maps that are dirty and need to be re-processed.
    /// Each entry should be processed before the next frame, as it is cleared at the end of each frame.
    pub(crate) dirty_render: Vec<Option<DirtyTile>>,
    pub(crate) dirty_physics: Vec<Option<DirtyTile>>,
}
impl Terrain {
    /// Initializes the terrain by loading the heightmaps and splat maps for each tile from the specified folder.
    /// 
    /// # Arguments
    /// * `path` - The path where the terrain assets are located (e.g., "assets/terrain")
    /// 
    /// # Returns
    /// A `Terrain` resource containing the initialized terrain tiles with their respective heightmap and splat map data.
    pub fn load(path: &str) -> Self {
        let mut pos_to_tile = HashMap::new();
        let mut tiles = Vec::new();
        let mut dirty_render = Vec::new();
        let mut dirty_physics = Vec::new();
        for i in 0..TERRAIN_TILES_COUNT {
            for j in 0..TERRAIN_TILES_COUNT {
                // Calculate the world position of the tile (centered around the origin)
                let px = i as i32 - (TERRAIN_TILES_COUNT as i32) / 2;
                let pz = j as i32 - (TERRAIN_TILES_COUNT as i32) / 2;
                let pos = IVec2::new(px, pz);

                // Compute files_names
                let cur_dir = std::env::current_dir().unwrap();
                let full_path = format!("{}/res/{}", cur_dir.display(), path);
                let heightmap_path = if std::fs::metadata(format!("{}/heightmap_{}_{}.png", full_path, px, pz)).is_ok() {
                    format!("{}/heightmap_{}_{}.png", full_path, px, pz)
                } else {
                    format!("{}/heightmap_default.png", full_path)
                };
                let mut splatmap_paths = Vec::new();
                for i in 0..SPLAT_MAP_COUNT / 4 {
                    let splatmap_path = if std::fs::metadata(format!("{}/splatmap_{}_{}-{}.png", full_path, px, pz, i)).is_ok() {
                        format!("{}/splatmap_{}_{}-{}.png", full_path, px, pz, i)
                    } else {
                        format!("{}/splatmap_default.png", full_path)
                    };
                    splatmap_paths.push(splatmap_path);
                }

                // Load the tile data from the files and create the tile
                let mut tile = TerrainTile {
                    pos,
                    heightmap: Vec::new(),
                    splatmaps: Vec::new(),
                };

                // Load heightmap as R8 (1 channel)
                tile.heightmap = match decode_png_as_channels(&heightmap_path, 1, (RENDER_TILE_SUBDIVISIONS, RENDER_TILE_SUBDIVISIONS)) {
                    Ok(data) => data,
                    Err(e) => {
                        error!("Failed to decode heightmap for tile ({}, {}): {}", pos.x, pos.y, e);
                        continue;
                    }
                };

                // Load splat maps
                for splatmap_path in splatmap_paths {
                    tile.splatmaps.push(match decode_png_as_channels(&splatmap_path, 4, (RENDER_TILE_SUBDIVISIONS, RENDER_TILE_SUBDIVISIONS)) {
                        Ok(data) => data,
                        Err(e) => {
                            error!("Failed to decode splatmap for tile ({}, {}): {}", pos.x, pos.y, e);
                            continue;
                        }
                    });
                }

                // Add the tile to the list and mark it as dirty
                dirty_render.push(Some((pos, 0, 0, tile.heightmap.clone()))); // Mark heightmap as dirty
                dirty_physics.push(Some((pos, 0, 0, tile.heightmap.clone())));
                for i in 0..SPLAT_MAP_COUNT / 4 {
                    dirty_render.push(Some((pos, 1, i, tile.splatmaps[i as usize].clone()))); // Mark splat map as dirty
                    dirty_physics.push(Some((pos, 1, i, tile.splatmaps[i as usize].clone())));
                }
                tiles.push(tile);
                pos_to_tile.insert(pos, tiles.len() - 1);
            }
        }
        Self {
            path: path.to_string(),
            pos_to_tile,
            dirty_render,
            dirty_physics
        }
    }

    /// Gets the tile indices for a given world position.
    /// 
    /// # Arguments
    /// * `world_pos` - The world position for which to get the tile indices
    /// 
    /// # Returns
    /// An option containing the tile indices (x, z) if the position is within the terrain bounds
    /// or None if the position is out of bounds.
    pub fn get_tile_idx_for_world_pos(&self, world_pos: Vec3) -> Option<IVec2> {
        let tile_x = (world_pos.x / TILE_SIZE[0]).round() as i32;
        let tile_z = (world_pos.z / TILE_SIZE[2]).round() as i32;
        let tile_pos = IVec2::new(tile_x, tile_z);
        if self.pos_to_tile.contains_key(&tile_pos) {
            Some(tile_pos)
        } else {
            None
        }
    }
}
