use bevy::prelude::*;
use wde_renderer::prelude::*;

/// The size of each terrain tile in world units (e.g., 100.0 means each tile covers 100x100 units)
pub(crate) const TILE_SIZE: f32 = 100.0;
/// The number of subdivisions per tile (e.g., 16 means each tile is divided into a 16x16 grid of vertices)
pub(crate) const TILE_SUBDIVISIONS: u32 = 256;
/// The number of splat maps per tile (must be a multiple of 4, as each splat map can store 4 channels for texture blending)
pub(crate) const SPLAT_MAP_COUNT: u32 = 4;

/// Represents a single terrain tile, containing its position and the associated heightmap, normal map, and splat maps.
#[derive(Default, Clone)]
pub(crate) struct TerrainRenderTile {
    // The position of the tile in world space (x, z)
    pub position: Vec2,

    // The heightmap, normal map, and splat maps for this tile
    pub heightmap: Handle<Texture>,
    pub normalmap: Handle<Texture>,
    pub splatmaps: Vec<Handle<Texture>>,

    // Is this tile dirty ?
    pub dirty: bool,
}

/// The main terrain resource that holds all the terrain tiles.
#[derive(Component)]
pub struct TerrainRenderer {
    pub(crate) tiles: Vec<TerrainRenderTile>
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
        for i in 0..terrain_size {
            for j in 0..terrain_size {
                // Calculate the world position of the tile (centered around the origin)
                let px = i - (terrain_size / 2);
                let pz = j - (terrain_size / 2);
                let position = Vec2::new(px as f32 * TILE_SIZE, pz as f32 * TILE_SIZE);

                // Load maps
                let heightmap = asset_server.load_with_settings(format!("{}/heightmap_{}_{}.png", path, px, pz), move |settings: &mut TextureLoaderSettings| {
                    settings.label = format!("heightmap_{}_{}", px, pz);
                    settings.format = TextureFormat::R8Unorm;
                    settings.usages = usages;
                });
                let normalmap = asset_server.load_with_settings(format!("{}/normalmap_{}_{}.png", path, px, pz), move |settings: &mut TextureLoaderSettings| {
                    settings.label = format!("normalmap_{}_{}", px, pz);
                    settings.format = TextureFormat::Rgba8Unorm;
                    settings.usages = usages;
                });
                let mut splatmaps = Vec::new();
                for i in 0..SPLAT_MAP_COUNT / 4 {
                    splatmaps.push(asset_server.load_with_settings(format!("{}/splatmap_{}_{}-{}.png", path, px, pz, i), move | settings: &mut TextureLoaderSettings| {
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
                    splatmaps,
                    dirty: true,
                });
            }
        }

        TerrainRenderer { tiles }
    }
}
