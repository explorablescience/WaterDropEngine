use wde_renderer::{core::MainWorld, prelude::*};
use bevy::prelude::*;

use crate::{manager::{TerrainDirtyTile, CHUNK_RENDER_SUBDIVISIONS, ChunkPos}, prelude::TerrainRendererGPU};

/// Type of the message sent from the render world to the main world when a tile is extracted from the GPU.
#[derive(Message)]
pub struct ExtractedTileMessage {
    pub pos: ChunkPos,
    pub map_type: u32, // 0 for heightmap, 1 for splatmap
    pub splat_map_index: u32, // only relevant if map_type is 1, should be between 0 and SPLAT_MAP_COUNT/4 - 1
    pub data: Vec<u8>
}

pub struct TerrainExtractorPlugin;
impl Plugin for TerrainExtractorPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<StaggingBuffers>()
            .init_resource::<TerrainExtractor>()
            .add_systems(Startup, init_stagging_buffers);
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<TerrainExtractor>()
            .init_resource::<StaggingBuffers>()
            .add_systems(Extract, extract_staging_buffers)
            .add_systems(Render, extract_tiles_from_gpu.in_set(RenderSet::Render));

        // Add the extract system
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Extract, extract_messages);
    }
}

/// Resource storing individual texture handles for each material
#[derive(Resource, Default)]
pub struct StaggingBuffers {
    pub heightmap: Option<Handle<Buffer>>,
    pub splatmap: Option<Handle<Buffer>>
}

#[derive(Resource, Default)]
pub struct TerrainExtractor {
    /// Liste of tiles that needs to be extracted from the gpu.
    /// First the tile position, then 0 for heightmap, 1 for splatmap, and then the index of the splatmap (0 to SPLAT_MAP_COUNT/4 - 1)
    tiles_to_extract: Vec<Option<(ChunkPos, u32, u32)>>,
    /// List of tiles that were extracted from the gpu and are ready to be sent to the main world
    extracted_tiles: Vec<Option<TerrainDirtyTile>>
}
impl TerrainExtractor {
    /// Queues a tile for extraction from the GPU.
    /// The tile will be processed in the next render pass and the extracted data will be available in the main world in the next frame.
    /// 
    /// # Arguments
    /// * `tile_pos` - The position of the tile to extract (x, z)
    /// * `map_type` - The type of map to extract (0 for heightmap, 1 for splatmap)
    /// * `splat_map_index` - The index of the splat map to extract (only relevant if map_type is 1, should be between 0 and SPLAT_MAP_COUNT/4 - 1)
    pub fn queue_tile_extraction(&mut self, tile_pos: ChunkPos, map_type: u32, splat_map_index: u32) {
        self.tiles_to_extract.push(Some((tile_pos, map_type, splat_map_index)));
    }

    /// Clears the list of tiles to extract. Should be called after processing the extracted tiles to avoid extracting the same tiles multiple times.
    pub fn clear_tiles_to_extract(&mut self) {
        self.tiles_to_extract.clear();
    }
}

/// Init stagging buffers
fn init_stagging_buffers(asset_server: Res<AssetServer>, mut buffers: ResMut<StaggingBuffers>) {
    // Build texture arrays for each type
    let usage = BufferUsage::COPY_DST | BufferUsage::MAP_READ;

    let stagging_heightmap = asset_server.add(Buffer {
        label: "stagging_heightmap".to_string(),
        size: (CHUNK_RENDER_SUBDIVISIONS as usize * CHUNK_RENDER_SUBDIVISIONS as usize) * std::mem::size_of::<u8>(),
        usage,
        content: None
    });
    let stagging_splatmap = asset_server.add(Buffer {
        label: "stagging_splatmap".to_string(),
        size: (CHUNK_RENDER_SUBDIVISIONS as usize * CHUNK_RENDER_SUBDIVISIONS as usize) * std::mem::size_of::<[u8; 4]>(), // RGBA8Unorm
        usage,
        content: None
    });

    buffers.heightmap = Some(stagging_heightmap);
    buffers.splatmap = Some(stagging_splatmap);
}

/// Extract the tiles data from the GPU
fn extract_tiles_from_gpu(
    mut extractor: ResMut<TerrainExtractor>,
    gpu_terrain_renderer: Res<TerrainRendererGPU>,
    textures: Res<RenderAssets<GpuTexture>>,
    render_instance: Res<RenderInstance>,
    mut buffers: ResMut<RenderAssets<GpuBuffer>>,
    stagging_buffers: Res<StaggingBuffers>
) {
    // Check if there are tiles to extract
    if extractor.tiles_to_extract.is_empty() {
        return;
    }

    // Process the tiles to extract
    let render_instance = render_instance.0.read().unwrap();
    for i in 0..extractor.tiles_to_extract.len() {
        if let Some((pos, map_type, splat_map_index)) = extractor.tiles_to_extract[i].take() {
            // Get the right texture handle
            let render_tile = &gpu_terrain_renderer.tiles[gpu_terrain_renderer.pos_to_tile[&pos]];
            let texture_handle = match map_type {
                0 => render_tile.heightmap,
                1 => render_tile.splatmaps[splat_map_index as usize],
                _ => continue,
            };

            // Get the GPU texture
            let texture = match textures.get(texture_handle) {
                Some(texture) => texture,
                None => continue,
            };

            // Get the buffer
            let staging_buffer_handle = match map_type {
                0 => &stagging_buffers.heightmap,
                1 => &stagging_buffers.splatmap,
                _ => continue,
            };
            let staging_buffer_handle = match staging_buffer_handle {
                Some(handle) => handle,
                None => continue,
            };
            let stagging_buffer = match buffers.get_mut(staging_buffer_handle) {
                 Some(buffer) => buffer,
                 None => continue,
             };
             
            // Copy the texture data to the staging buffer
            stagging_buffer.buffer.copy_from_texture(&render_instance, &texture.texture.texture);

            // Read the buffer data and create a dirty tile
            let mut data = Vec::new();
            stagging_buffer.buffer.map_read(&render_instance, |view| {
                data = view.to_vec();
            });

            // Create the dirty tile and add it to the extracted tiles list
            extractor.extracted_tiles.push(Some((
                pos,
                if map_type == 0 { 0 } else { 1 },
                if map_type == 1 { splat_map_index } else { 0 },
                data
            )));
            extractor.tiles_to_extract[i] = None;
        }
    }

    // Remove None entries from the tiles to extract list
    extractor.tiles_to_extract.retain(|entry| entry.is_some());
}


fn extract_staging_buffers(
    main_stagging_buffers: ExtractWorld<Res<StaggingBuffers>>,
    mut render_stagging_buffers: ResMut<StaggingBuffers>
) {
    if render_stagging_buffers.heightmap.is_some() {
        return;
    }

    if let (Some(heightmap), Some(splatmap))
        = (main_stagging_buffers.heightmap.clone(), main_stagging_buffers.splatmap.clone()) {
        render_stagging_buffers.heightmap = Some(heightmap);
        render_stagging_buffers.splatmap = Some(splatmap);
    }
}

pub fn extract_dirty(
    main_terrain_extractor: &TerrainExtractor,
    render_terrain_extractor: &mut TerrainExtractor
) {
    // Extract the tiles_to_extract from the main world and move them to the render world extractor resource
    render_terrain_extractor.tiles_to_extract = main_terrain_extractor.tiles_to_extract.clone();
}

pub fn extract_messages(
    mut render_terrain_extractor: ResMut<TerrainExtractor>,
    mut main_world: ResMut<MainWorld>
) {
    // Send events for each extracted tile to the main world
    for (pos, map_type, splat_map_index, data) in render_terrain_extractor.extracted_tiles.clone().into_iter().flatten() {
        main_world.write_message(ExtractedTileMessage { pos, map_type, splat_map_index, data });
    }

    // Clear the extracted tiles list after processing
    render_terrain_extractor.extracted_tiles.clear();
}

