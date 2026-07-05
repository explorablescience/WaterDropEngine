use bevy::prelude::*;
use wde_renderer::{prelude::*, wgpu_utils::AsyncReadback};

use crate::{
    manager::ChunkPos,
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

/// System sets for ordering terrain render-world work.
/// External plugins that write terrain textures (e.g. `wde-terrain-editor`) should place
/// their compute dispatch systems inside [`TerrainRenderSets::TextureWrite`].
/// [`initiate_tile_readbacks`] is configured to run **after** this set so it always
/// captures post-compute texture data.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TerrainRenderSets {
    /// Any compute pass that writes to the terrain heightmap or splatmap textures.
    TextureWrite
}

pub struct TerrainExtractorPlugin;
impl Plugin for TerrainExtractorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainExtractor>();

        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        render_app
            .init_resource::<TerrainExtractor>()
            .init_resource::<PendingReadbacks>()
            // TextureWrite set must finish before tile readbacks start.
            .configure_sets(
                Render,
                TerrainRenderSets::TextureWrite.before(initiate_tile_readbacks)
            )
            // Poll pending readbacks and forward completions to the main world.
            .add_systems(Extract, flush_completed_readbacks)
            // Kick off GPU copies for the tiles queued this frame.
            .add_systems(
                Render,
                initiate_tile_readbacks.in_set(RenderSet::Render)
            );
    }
}

/// Queue of tiles to read back from the GPU.
/// Lives in both the main world (as a write target for game systems) and the render world
/// (consumed by [`initiate_tile_readbacks`]).
#[derive(Resource, Default)]
pub struct TerrainExtractor {
    pub tiles_to_extract: Vec<(ChunkPos, u32, u32)>
}

impl TerrainExtractor {
    pub fn queue_tile_extraction(&mut self, pos: ChunkPos, map_type: u32, splat_index: u32) {
        self.tiles_to_extract.push((pos, map_type, splat_index));
    }
}

/// In-flight GPU→CPU readback for one tile.
struct PendingReadback {
    pos: ChunkPos,
    map_type: u32,
    splat_index: u32,
    readback: AsyncReadback
}

/// Render-world store of all in-flight readbacks.
#[derive(Resource, Default)]
pub struct PendingReadbacks {
    pending: Vec<PendingReadback>
}

/// Transfer the main world's extraction queue into the render world.
/// Called by [`TerrainRendererGPU::extract_dirty`] during the Extract phase.
pub fn extract_dirty(main: &mut TerrainExtractor, render: &mut TerrainExtractor) {
    render.tiles_to_extract = std::mem::take(&mut main.tiles_to_extract);
}

/// Render phase: copy each queued tile to a staging buffer and start async mapping.
/// Must run **after** the paint compute pass so the copy captures post-compute data.
/// This ordering is enforced in [`PaintProcessorPlugin`].
pub fn initiate_tile_readbacks(
    mut extractor: ResMut<TerrainExtractor>,
    mut pending: ResMut<PendingReadbacks>,
    gpu: Res<TerrainRendererGPU>,
    textures: Res<RenderAssets<GpuTexture>>,
    render_instance: Res<RenderInstance>
) {
    if extractor.tiles_to_extract.is_empty() {
        return;
    }

    let ri = render_instance.0.read().unwrap();
    let tiles = std::mem::take(&mut extractor.tiles_to_extract);

    for (pos, map_type, splat_index) in tiles {
        let Some(&layer) = gpu.pos_to_layer.get(&pos) else {
            continue;
        };
        let texture_handle = match map_type {
            0 => gpu.heightmap_array.as_ref(),
            1 => gpu.splatmap_array.as_ref(),
            _ => continue
        };
        let Some(tex_h) = texture_handle else { continue };
        let Some(tex) = textures.get(tex_h) else { continue };

        pending.pending.push(PendingReadback {
            pos,
            map_type,
            splat_index,
            readback: AsyncReadback::from_texture_layer(&ri, &tex.texture.texture, layer)
        });
    }
}

/// Extract phase: non-blocking poll + collect any readbacks the GPU completed last frame.
/// Forwards completed tile data to the main world as [`ExtractedTileMessage`]s.
fn flush_completed_readbacks(
    mut pending: ResMut<PendingReadbacks>,
    render_instance: Res<RenderInstance>,
    mut main_world: ResMut<MainWorld>
) {
    if pending.pending.is_empty() {
        return;
    }

    // Non-blocking poll: triggers any map_async callbacks the GPU has finished.
    render_instance.0.read().unwrap().poll_non_blocking();

    let drained = std::mem::take(&mut pending.pending);
    for PendingReadback { pos, map_type, splat_index, readback } in drained {
        match readback.try_collect() {
            Ok(data) if !data.is_empty() => {
                main_world.write_message(ExtractedTileMessage {
                    pos,
                    map_type,
                    splat_map_index: splat_index,
                    data
                });
            }
            Ok(_) => {} // empty data means a mapping error was already logged
            Err(readback) => {
                // Mapping not done yet — return to queue for next frame.
                pending.pending.push(PendingReadback { pos, map_type, splat_index, readback });
            }
        }
    }
}
