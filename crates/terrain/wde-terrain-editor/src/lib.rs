//! Terrain editor for WaterDropEngine.
//!
//! Gives each spawned terrain (see `wde-terrain`) a paint brush, and edits it on the GPU:
//! the brush turns mouse strokes on the terrain into paint commands, run by a compute pass on the
//! chunk maps. Painting modes edit the splat maps (paint, erase), the others the heightmaps (raise,
//! lower, smooth, flatten). Edited chunks are read back to update the terrain heights and
//! colliders.
//!
//! The editor windows are in the `Terrain` menu:
//! - `Terrain/Paint`: brush mode, radius, strength and color.
//! - `Terrain/Settings`: tiling and height blending of the ground materials.
//! - `Terrain/Save`: writes the edited maps back to the PNG files of the terrain in `res/`.
//!
//! The `Debug/Terrain` window tints the terrain with its masks (buildable, slope), and reads them
//! under the cursor.
use wde_logger::prelude::*;

use bevy::prelude::*;
use wde_terrain::prelude::*;

use crate::{
    paint::{brush::PaintBrush, paint_manager::PaintManagerPlugin},
    processor::PaintProcessorPlugin,
    ui_painter::TerrainEditorUIPlugin
};

mod paint;
mod processor;
mod ui_painter;

pub struct TerrainEditorPlugin;
impl Plugin for TerrainEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PaintManagerPlugin,
            PaintProcessorPlugin,
            TerrainEditorUIPlugin
        ))
        .init_resource::<SaveManager>()
        .add_systems(Update, add_brush)
        .add_systems(Update, handle_extracted_tiles)
        .add_message::<ExtractedTileMessage>();
    }
}

fn add_brush(mut commands: Commands, terrains: Query<Entity, Added<Terrain>>) {
    for terrain in &terrains {
        commands.spawn((
            Name::new("Terrain Default Brush"),
            PaintBrush::default(),
            ChildOf(terrain)
        ));
    }
}

#[derive(Resource, Default)]
struct SaveManager {
    saving: bool,
    tiles_to_save: Vec<(ChunkPos, u32, u32)>
}

/// Handle all tile readbacks from the GPU:
/// - Always update physics and the CPU heights when a heightmap arrives.
/// - When in save mode, also write the tile to disk (without its border, as loaded).
fn handle_extracted_tiles(
    mut terrain: Query<&mut Terrain>,
    mut save_manager: ResMut<SaveManager>,
    mut extracted_tiles: MessageReader<ExtractedTileMessage>
) {
    let mut terrain = match terrain.single_mut() {
        Ok(t) => t,
        Err(_) => return
    };

    for message in extracted_tiles.read() {
        let ExtractedTileMessage {
            pos,
            map_type,
            splat_map_index,
            data
        } = message;

        if *map_type == 0 {
            terrain.dirty_physics.push((*pos, 0, 0, data.clone()));
            terrain.set_chunk_heights(*pos, data);
        }

        // Optionally save to disk.
        if save_manager.saving {
            let folder = Terrain::folder(&terrain.path);
            let result = match map_type {
                0 => save_heightmap(
                    &format!("{folder}/{}", terrain.config.heightmap_file(*pos)),
                    data
                ),
                1 => save_splatmap(
                    &format!(
                        "{folder}/{}",
                        terrain.config.splatmap_file(*pos, *splat_map_index)
                    ),
                    data
                ),
                _ => continue
            };
            if let Err(e) = result {
                error!("Failed to save terrain tile ({}, {}): {}", pos.x, pos.y, e);
            }
            save_manager
                .tiles_to_save
                .retain(|(p, t, s)| !(p == pos && t == map_type && s == splat_map_index));
        }
    }

    if save_manager.saving && save_manager.tiles_to_save.is_empty() {
        info!("Finished saving terrain.");
        save_manager.saving = false;
    }
}

/// Saves the chunk samples of a GPU heightmap as a 16-bit grayscale PNG.
fn save_heightmap(path: &str, map: &[u8]) -> Result<(), String> {
    let data: Vec<u8> = chunk_samples(map)
        .iter()
        .flat_map(|h| ((h.clamp(0.0, 1.0) * u16::MAX as f32).round() as u16).to_be_bytes())
        .collect();
    write_png(
        path,
        &data,
        png::ColorType::Grayscale,
        png::BitDepth::Sixteen
    )
}

/// Saves the chunk samples of a GPU splat map as an RGBA PNG.
fn save_splatmap(path: &str, map: &[u8]) -> Result<(), String> {
    let size = CHUNK_MAP_SIZE as usize;
    let data: Vec<u8> = (1..size - 1)
        .flat_map(|z| (1..size - 1).map(move |x| (z * size + x) * 4))
        .flat_map(|i| map[i..i + 4].to_vec())
        .collect();
    write_png(path, &data, png::ColorType::Rgba, png::BitDepth::Eight)
}

fn write_png(
    path: &str,
    data: &[u8],
    color: png::ColorType,
    depth: png::BitDepth
) -> Result<(), String> {
    let file =
        std::fs::File::create(path).map_err(|e| format!("Failed to create file {path}: {e}"))?;
    let mut encoder =
        png::Encoder::new(std::io::BufWriter::new(file), CHUNK_SAMPLES, CHUNK_SAMPLES);
    encoder.set_color(color);
    encoder.set_depth(depth);
    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("Failed to write PNG header: {e}"))?;
    writer
        .write_image_data(data)
        .map_err(|e| format!("Failed to write PNG data: {e}"))
}
