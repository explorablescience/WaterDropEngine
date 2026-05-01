use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::{
    manager::{CHUNK_RENDER_SUBDIVISIONS, ChunkPos, TerrainDirtyTile},
    prelude::TerrainRendererGPU
};

/// Message sent from the render world to the main world when a tile is read back from GPU.
#[derive(Message)]
pub struct ExtractedTileMessage {
    pub pos: ChunkPos,
    pub map_type: u32,        // 0 = heightmap, 1 = splatmap
    pub splat_map_index: u32, // only relevant when map_type == 1
    pub data: Vec<u8>
}

pub struct TerrainExtractorPlugin;
impl Plugin for TerrainExtractorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StagingBuffers>()
            .init_resource::<TerrainExtractor>()
            .add_systems(Startup, init_staging_buffers);

        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        render_app
            .init_resource::<TerrainExtractor>()
            .init_resource::<StagingBuffers>()
            .add_systems(Extract, extract_staging_buffers)
            .add_systems(Extract, extract_messages)
            .add_systems(Render, extract_tiles_from_gpu.in_set(RenderSet::Render));
    }
}

/// Staging buffers for GPU → CPU readback.
#[derive(Resource, Default)]
pub struct StagingBuffers {
    pub heightmap: Option<Handle<Buffer>>,
    pub splatmap: Option<Handle<Buffer>>
}

/// Queue of tiles to read back from the GPU, shared between main and render worlds.
#[derive(Resource, Default)]
pub struct TerrainExtractor {
    /// Tiles queued for GPU extraction.
    pub tiles_to_extract: Vec<(ChunkPos, u32, u32)>,
    /// Tiles already extracted and ready to send to the main world.
    extracted_tiles: Vec<TerrainDirtyTile>
}

impl TerrainExtractor {
    /// Queue a tile for GPU readback.
    pub fn queue_tile_extraction(&mut self, pos: ChunkPos, map_type: u32, splat_index: u32) {
        self.tiles_to_extract.push((pos, map_type, splat_index));
    }
}

fn init_staging_buffers(asset_server: Res<AssetServer>, mut staging: ResMut<StagingBuffers>) {
    let usage = BufferUsage::COPY_DST | BufferUsage::MAP_READ;
    let px = CHUNK_RENDER_SUBDIVISIONS as usize;

    staging.heightmap = Some(asset_server.add(Buffer {
        label: "staging_heightmap".to_string(),
        size: px * px * std::mem::size_of::<u8>(),
        usage,
        content: None
    }));
    staging.splatmap = Some(asset_server.add(Buffer {
        label: "staging_splatmap".to_string(),
        size: px * px * std::mem::size_of::<[u8; 4]>(),
        usage,
        content: None
    }));
}

/// Drain the main world's extraction queue into the render world's queue.
pub fn extract_dirty(main: &mut TerrainExtractor, render: &mut TerrainExtractor) {
    render.tiles_to_extract = std::mem::take(&mut main.tiles_to_extract);
}

fn extract_staging_buffers(
    main: ExtractWorld<Res<StagingBuffers>>,
    mut render: ResMut<StagingBuffers>
) {
    if render.heightmap.is_some() {
        return;
    }
    if let (Some(h), Some(s)) = (main.heightmap.clone(), main.splatmap.clone()) {
        render.heightmap = Some(h);
        render.splatmap = Some(s);
    }
}

fn extract_tiles_from_gpu(
    mut extractor: ResMut<TerrainExtractor>,
    gpu: Res<TerrainRendererGPU>,
    textures: Res<RenderAssets<GpuTexture>>,
    render_instance: Res<RenderInstance>,
    mut buffers: ResMut<RenderAssets<GpuBuffer>>,
    staging: Res<StagingBuffers>
) {
    if extractor.tiles_to_extract.is_empty() {
        return;
    }

    let render_instance = render_instance.0.read().unwrap();
    let tiles = std::mem::take(&mut extractor.tiles_to_extract);

    for (pos, map_type, splat_index) in tiles {
        let layer = match gpu.pos_to_layer.get(&pos) {
            Some(&l) => l,
            None => continue
        };

        let (texture_handle, staging_handle) = match map_type {
            0 => (gpu.heightmap_array.as_ref(), staging.heightmap.as_ref()),
            1 => (gpu.splatmap_array.as_ref(), staging.splatmap.as_ref()),
            _ => continue
        };
        let (Some(tex_h), Some(stag_h)) = (texture_handle, staging_handle) else {
            continue;
        };
        let tex = match textures.get(tex_h) {
            Some(t) => t,
            None => continue
        };
        let staging_buf = match buffers.get_mut(stag_h) {
            Some(b) => b,
            None => continue
        };

        staging_buf
            .buffer
            .copy_from_texture_layered(&render_instance, &tex.texture.texture, layer);

        let mut data = Vec::new();
        staging_buf.buffer.map_read(&render_instance, |view| {
            data = view.to_vec();
        });

        extractor
            .extracted_tiles
            .push((pos, map_type, splat_index, data));
    }
}

fn extract_messages(
    mut render_extractor: ResMut<TerrainExtractor>,
    mut main_world: ResMut<MainWorld>
) {
    for (pos, map_type, splat_map_index, data) in
        std::mem::take(&mut render_extractor.extracted_tiles)
    {
        main_world.write_message(ExtractedTileMessage {
            pos,
            map_type,
            splat_map_index,
            data
        });
    }
}
