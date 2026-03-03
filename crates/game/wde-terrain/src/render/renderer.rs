use std::collections::HashMap;

use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::manager::SPLAT_MAP_COUNT;

/// Represents a single terrain tile, containing its position and the associated heightmap, normal map, and splat maps.
#[derive(Default, Clone)]
pub(crate) struct TerrainRenderTile {
    // The position of the tile in world space (x, z)
    pub position: IVec2,

    // The heightmap, normal map, and splat maps for this tile
    pub heightmap: Handle<Texture>,
    pub normalmap: Handle<Texture>,
    pub splatmaps: Vec<Handle<Texture>>
}

/// The main terrain resource that holds all the terrain tiles.
#[derive(Component)]
pub struct TerrainRenderer {
    // Pointer from position (x, z) to the corresponding tile datas
    pub tile_map: HashMap<IVec2, usize>,
    // A list of terrain tiles that make up the entire terrain
    pub tiles: Vec<TerrainRenderTile>,

    // The list of indices that are dirty and need to be re-uploaded to the GPU
    pub dirty_tiles: Vec<IVec2>
}
impl TerrainRenderer {
    /// Initializes the terrain by loading the heightmaps, normal maps, and splat maps for each tile from the specified folder.
    /// 
    /// # Arguments
    /// * `path` - The path where the terrain assets are located (e.g., "assets/terrain")
    /// * `terrain_size` - The number of tiles along one axis (e.g., 4 means a 4x4 grid of tiles)
    /// * `asset_server` - The Bevy asset server used to load the textures
    /// 
    /// # Returns
    /// A `Terrain` resource containing the initialized terrain tiles with their respective textures.
    pub fn new(path: &str, terrain_size: usize, asset_server: &AssetServer) -> Self {
        let usages = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
        let mut tiles = Vec::new();
        let mut tile_map = HashMap::new();
        let mut dirty_tiles = Vec::new();
        for i in 0..terrain_size {
            for j in 0..terrain_size {
                // Calculate the world position of the tile (centered around the origin)
                let px = i as i32 - (terrain_size as i32 / 2);
                let pz = j as i32 - (terrain_size as i32 / 2);
                let position = IVec2::new(px, pz);

                // Compute files_names
                let cur_dir = std::env::current_dir().unwrap();
                let full_path = format!("{}/res/{}", cur_dir.display(), path);
                let heightmap_path = if std::fs::metadata(format!("{}/heightmap_{}_{}.png", full_path, px, pz)).is_ok() {
                    format!("{}/heightmap_{}_{}.png", path, px, pz)
                } else {
                    format!("{}/heightmap_default.png", path)
                };
                let normalmap_path = if std::fs::metadata(format!("{}/normalmap_{}_{}.png", full_path, px, pz)).is_ok() {
                    format!("{}/normalmap_{}_{}.png", path, px, pz)
                } else {
                    format!("{}/normalmap_default.png", path)
                };
                let mut splatmap_paths = Vec::new();
                for i in 0..SPLAT_MAP_COUNT / 4 {
                    let splatmap_path = if std::fs::metadata(format!("{}/splatmap_{}_{}-{}.png", full_path, px, pz, i)).is_ok() {
                        format!("{}/splatmap_{}_{}-{}.png", path, px, pz, i)
                    } else {
                        format!("{}/splatmap_default.png", path)
                    };
                    splatmap_paths.push(splatmap_path);
                }

                // Load maps
                let heightmap = asset_server.load_with_settings(heightmap_path, move |settings: &mut TextureLoaderSettings| {
                    settings.label = format!("heightmap_{}_{}", px, pz);
                    settings.format = TextureFormat::R8Unorm;
                    settings.usages = usages;
                });
                let normalmap = asset_server.load_with_settings(normalmap_path, move |settings: &mut TextureLoaderSettings| {
                    settings.label = format!("normalmap_{}_{}", px, pz);
                    settings.format = TextureFormat::Rgba8Unorm;
                    settings.usages = usages;
                });
                let mut splatmaps = Vec::new();
                for i in 0..SPLAT_MAP_COUNT / 4 {
                    let splatmap_path = &splatmap_paths[i as usize];
                    splatmaps.push(asset_server.load_with_settings(splatmap_path, move | settings: &mut TextureLoaderSettings| {
                        settings.label = format!("splatmap_{}_{}_{}", px, pz, i);
                        settings.format = TextureFormat::Rgba8Unorm;
                        settings.usages = usages;
                    }));
                }

                // Create the tile and add it to the list
                tiles.push(TerrainRenderTile {
                    position,
                    heightmap,
                    normalmap,
                    splatmaps
                });
                tile_map.insert(position, tiles.len() - 1);
                dirty_tiles.push(position);
            }
        }
        TerrainRenderer { dirty_tiles, tile_map, tiles }
    }
}
