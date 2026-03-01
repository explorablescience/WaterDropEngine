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
pub(crate) struct TerrainTile {
    // The position of the tile in world space (x, z)
    pub position: Vec2,

    // The heightmap, normal map, and splat maps for this tile
    pub heightmap: Handle<Texture>,
    pub normalmap: Handle<Texture>,
    pub splatmaps: Vec<Handle<Texture>>,

    // The bind group for the different maps
    pub bind_group_layout: Option<BindGroupLayout>,
    pub bind_group: Option<BindGroup>
}

/// The main terrain resource that holds all the terrain tiles.
#[derive(Resource, Default, Clone)]
pub struct Terrain {
    pub(crate) tiles: Vec<TerrainTile>
}
impl Terrain {
    /// Initializes the terrain by loading the heightmaps, normal maps, and splat maps for each tile from the specified folder.
    /// 
    /// # Arguments
    /// * `terrain_size` - The number of tiles along one axis (e.g., 4 means a 4x4 grid of tiles)
    /// * `path` - The path where the terrain assets are located (e.g., "assets/terrain")
    /// * `asset_server` - The Bevy asset server used to load the textures
    /// 
    /// # Returns
    /// A `Terrain` resource containing the initialized terrain tiles with their respective textures.
    pub fn init(terrain_size: usize, path: &str, asset_server: &AssetServer) -> Self {
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
                tiles.push(TerrainTile {
                    position,
                    heightmap,
                    normalmap,
                    splatmaps,
                    bind_group_layout: None,
                    bind_group: None
                });
            }
        }

        Terrain { tiles }
    }

    /// Build the bind group for the deferred renderer.
    pub fn build_bind_group(mut textures: ResMut<RenderAssets<GpuTexture>>, render_instance: Res<RenderInstance>, mut terrain: ResMut<Terrain>) {
        let render_instance = render_instance.0.read().unwrap();
        for tile in &mut terrain.tiles {
            // Get the maps
            let (heightmap, normalmap, splatmaps) = match (
                textures.get(&tile.heightmap),
                textures.get(&tile.normalmap),
                tile.splatmaps.iter().map(|splatmap| textures.get(splatmap)).collect::<Option<Vec<_>>>(),
            ) {
                (Some(heightmap), Some(normalmap), Some(splatmaps)) => (heightmap, normalmap, splatmaps),
                _ => continue,
            };

            // Check if any of the textures is dirty, if so we need to rebuild the bind group
            if cfg!(debug_assertions)
                && !heightmap.dirty && !normalmap.dirty
                && splatmaps.iter().all(|splatmap| !splatmap.dirty) {
                continue;
            }

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
                textures.get_mut(&tile.heightmap).unwrap().dirty = false;
                textures.get_mut(&tile.normalmap).unwrap().dirty = false;
                for splatmap in &tile.splatmaps {
                    textures.get_mut(splatmap).unwrap().dirty = false;
                }
            }

            // Insert the resources
            tile.bind_group_layout = Some(bind_group_layout);
            tile.bind_group = Some(bind_group);
        }
    }
}
