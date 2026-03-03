use std::collections::HashMap;

use wde_logger::prelude::*;
use bevy::prelude::*;

use crate::utils::image_decoder::decode_png_as_channels;

/// The size of the terrain in terms of number of tiles (e.g., 4 means the terrain is made up of a 4x4 grid of tiles)
pub const TERRAIN_TILES_COUNT: usize = 8;
/// The size of each terrain tile in world units
pub const TILE_SIZE: [f32; 3] = [100.0, 20.0, 100.0];

/// The number of splat maps per tile (must be a multiple of 4, as each splat map can store 4 channels for texture blending)
pub const SPLAT_MAP_COUNT: u32 = 4;
/// The number of subdivisions per tile (e.g., 16 means each tile is divided into a 16x16 grid of vertices)
/// This should match the width and height of the texture maps.
pub const RENDER_TILE_SUBDIVISIONS: u32 = 256;

/// The size of the heightmap subdivisions per tile (e.g., 16 means the heightmap is divided into a 16x16 grid of height samples)
pub const PHYSICS_TILE_SUBDIVISIONS: u32 = 256;

// The structure describing the 2d position of a given terrain tile.
pub type TilePos = IVec2;
// The raw data of a terrain tile, containing the heightmap and splat maps as byte arrays.
pub type TileData = Vec<f32>;
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
    /// Pointer from tile position (x, z) to the corresponding tile datas
    pub pos_to_tile: HashMap<TilePos, usize>,
    /// A list of terrain tiles that make up the entire terrain. This is the source of truth for the terrain data.
    pub tiles: Vec<TerrainTile>,

    /// List of tile maps that are dirty and need to be re-processed.
    /// Each entry should be processed before the next frame, as it is cleared at the end of each frame.
    pub(crate) dirty: Vec<DirtyTile>
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
        let mut dirty = Vec::new();
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
                let data_as_u8 = match decode_png_as_channels(&heightmap_path, 1, (RENDER_TILE_SUBDIVISIONS, RENDER_TILE_SUBDIVISIONS)) {
                    Ok(data) => data,
                    Err(e) => {
                        error!("Failed to decode heightmap for tile ({}, {}): {}", pos.x, pos.y, e);
                        continue;
                    }
                };
                let ss = PHYSICS_TILE_SUBDIVISIONS;
                let mut heights = vec![0.0; ss as usize * ss as usize];
                for i in 0..ss {
                    for j in 0..ss {
                        let idx = (i * ss + j) as usize;
                        heights[idx] = data_as_u8[idx] as f32 / 255.0;
                    }
                }
                tile.heightmap = heights;

                // Load splat maps
                for splatmap_path in splatmap_paths {
                    let data_as_u8 = match decode_png_as_channels(&splatmap_path, 4, (RENDER_TILE_SUBDIVISIONS, RENDER_TILE_SUBDIVISIONS)) {
                        Ok(data) => data,
                        Err(e) => {
                            error!("Failed to decode splatmap for tile ({}, {}): {}", pos.x, pos.y, e);
                            continue;
                        }
                    };
                    let ss = PHYSICS_TILE_SUBDIVISIONS;
                    let mut data_f32 = vec![0.0; ss as usize * ss as usize * 4];
                    for i in 0..ss {
                        for j in 0..ss {
                            for k in 0..4 {
                                let idx = ((i * ss + j) * 4 + k) as usize;
                                data_f32[idx] = data_as_u8[idx] as f32 / 255.0;
                            }
                        }
                    }
                    tile.splatmaps.push(data_f32);
                }

                // Add the tile to the list and mark it as dirty
                dirty.push((pos, 0, 0, tile.heightmap.clone())); // Mark heightmap as dirty
                for i in 0..SPLAT_MAP_COUNT / 4 {
                    dirty.push((pos, 1, i, tile.splatmaps[i as usize].clone())); // Mark splat map as dirty
                }
                tiles.push(tile);
                pos_to_tile.insert(pos, tiles.len() - 1);
            }
        }
        Self {
            pos_to_tile,
            tiles,
            dirty
        }
    }

    /// Clears the dirty tiles list after they have been processed.
    /// This is called at PostUpdate every frame.
    pub fn clear_dirty(mut terrain: Query<&mut Terrain>) {
        let mut terrain = match terrain.iter_mut().next() {
            Some(terrain) => terrain,
            None => return,
        };
        terrain.dirty.clear();
    }
}
