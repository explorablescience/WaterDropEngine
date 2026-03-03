use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::{manager::SPLAT_MAP_COUNT, render::renderer::TerrainRenderer};

#[derive(Default, Clone)]
pub struct ExtractedTerrainTile {
    // The position of the tile in world space (x, z)
    pub position: IVec2,

    // The heightmap, normal map, and splat maps for this tile
    pub heightmap: AssetId<Texture>,
    pub normalmap: AssetId<Texture>,
    pub splatmaps: Vec<AssetId<Texture>>,

    // The bind group for the different maps
    pub bind_group_layout: Option<BindGroupLayout>,
    pub bind_group: Option<BindGroup>
}

#[derive(Resource, Default)]
pub struct GpuTerrainTiles {
    // The tiles that are ready to be rendered (i.e., have their textures loaded and bind groups created)
    pub ready_tiles: Vec<ExtractedTerrainTile>,
    // The tiles that were extracted from the main world but may not be ready for rendering yet (e.g., waiting for textures to load)
    pub extracted_tiles: Vec<ExtractedTerrainTile>
}
impl GpuTerrainTiles {
    // Extract the terrain tiles from the main world. This should be called in the Extract stage.
    pub fn extract_tiles(main_world: &mut World, render_world: &mut World) {
        // Get the terrain renderer resource and the GPU terrain tiles resource
        let mut terrain = match main_world.query::<&mut TerrainRenderer>().iter_mut(main_world).next() {
            Some(terrain) => terrain,
            None => return,
        };
        let mut gpu_terrain_tiles = render_world
            .get_resource_mut::<GpuTerrainTiles>()
            .unwrap();

        // Extract the dirty tiles
        for tile_pos in &terrain.dirty_tiles {
            let tile_index = match terrain.tile_map.get(tile_pos) {
                Some(index) => *index,
                None => continue,
            };
            let tile = &terrain.tiles[tile_index];
            let position = tile.position;

            // Process new tiles
            gpu_terrain_tiles.extracted_tiles.push(ExtractedTerrainTile {
                position,
                heightmap: tile.heightmap.id(),
                normalmap: tile.normalmap.id(),
                splatmaps: tile.splatmaps.iter().map(|splatmap| splatmap.id()).collect(),
                bind_group_layout: None,
                bind_group: None
            });
        }
        terrain.dirty_tiles.clear();
    }

    // Prepare the bind groups and layouts of the newly extracted tiles, and move them to the ready list once they are ready.
    pub fn prepare_tiles(mut gpu_terrain_tiles: ResMut<GpuTerrainTiles>, mut textures: ResMut<RenderAssets<GpuTexture>>, render_instance: Res<RenderInstance>) {
        let render_instance = render_instance.0.read().unwrap();
        for tile in &mut gpu_terrain_tiles.extracted_tiles {
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
        while i < gpu_terrain_tiles.extracted_tiles.len() {
            if gpu_terrain_tiles.extracted_tiles[i].bind_group.is_some() {
                let tile = gpu_terrain_tiles.extracted_tiles.remove(i);
                gpu_terrain_tiles.ready_tiles.push(tile);
            } else {
                i += 1;
            }
        }
    }

    /// Check if any texture of any tile has been modified (e.g., file changed on disk). If so, remove the tile from the ready list and add it back to the extracted list to be re-prepared.
    pub fn check_dirty_tiles(mut gpu_terrain_tiles: ResMut<GpuTerrainTiles>, textures: Res<RenderAssets<GpuTexture>>) {
        let mut i = 0;
        while i < gpu_terrain_tiles.ready_tiles.len() {
            let tile = &gpu_terrain_tiles.ready_tiles[i];
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
                let tile = gpu_terrain_tiles.ready_tiles.remove(i);
                gpu_terrain_tiles.extracted_tiles.push(tile);
            } else {
                i += 1;
            }
        }
    }
}
