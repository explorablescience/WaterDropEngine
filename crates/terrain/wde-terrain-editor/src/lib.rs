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

pub mod prelude {
    pub use super::TerrainEditorPlugin;
}

pub struct TerrainEditorPlugin;
impl Plugin for TerrainEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PaintManagerPlugin)
            .add_plugins(PaintProcessorPlugin)
            .add_plugins(TerrainEditorUIPlugin)
            .init_resource::<SaveManager>()
            .add_systems(Startup, init)
            .add_systems(Update, save_extracted_tiles)
            .add_message::<ExtractedTileMessage>();
    }
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn a terrain
    commands.spawn((
        Name::new("Terrain"),
        Terrain::load("tests/terrain"),
        TerrainRenderer::new(&asset_server),
        TerrainPhysics::default()
    ));

    // Spawn a default brush for testing
    commands.spawn((Name::new("Terrain Default Brush"), PaintBrush::default()));
}

#[derive(Resource, Default)]
struct SaveManager {
    // True if we are currently saving the terrain
    saving: bool,
    // List of tiles to save, with their position and map type (0 for heightmap, 1 for splatmap)
    tiles_to_save: Vec<(ChunkPos, u32, u32)>
}

fn save_extracted_tiles(
    terrain: Query<&Terrain>,
    mut save_manager: ResMut<SaveManager>,
    mut extracted_tiles: MessageReader<ExtractedTileMessage>
) {
    // If we are not currently saving, ignore the extracted tiles
    if !save_manager.saving {
        return;
    }

    // Get the terrain from main world
    let terrain = match terrain.single() {
        Ok(terrain) => terrain,
        Err(_) => return
    };

    // Process each extracted tile message
    for message in extracted_tiles.read() {
        let ExtractedTileMessage {
            pos,
            map_type,
            splat_map_index,
            data
        } = message;

        // Save the extracted tile data to a file
        let cur_dir = std::env::current_dir().unwrap();
        let full_path = format!("{}/res/{}", cur_dir.display(), terrain.path);
        match map_type {
            0 => {
                let path = format!("{}/heightmap_{}_{}.png", full_path, pos.x, pos.y);
                if let Err(e) = save_png_from_channels(
                    &path,
                    data,
                    1,
                    (CHUNK_RENDER_SUBDIVISIONS, CHUNK_RENDER_SUBDIVISIONS)
                ) {
                    error!(
                        "Failed to save heightmap for tile ({}, {}): {}",
                        pos.x, pos.y, e
                    );
                }
            }
            1 => {
                let path = format!(
                    "{}/splatmap_{}_{}-{}.png",
                    full_path, pos.x, pos.y, splat_map_index
                );
                if let Err(e) = save_png_from_channels(
                    &path,
                    data,
                    4,
                    (CHUNK_RENDER_SUBDIVISIONS, CHUNK_RENDER_SUBDIVISIONS)
                ) {
                    error!(
                        "Failed to save splatmap for tile ({}, {}): {}",
                        pos.x, pos.y, e
                    );
                }
            }
            _ => continue
        };
        save_manager
            .tiles_to_save
            .retain(|(p, t, s)| !(p == pos && t == map_type && s == splat_map_index));
    }

    // If all tiles have been saved, mark saving as false
    if save_manager.tiles_to_save.is_empty() {
        info!("Finished saving terrain.");
        save_manager.saving = false;
    }
}

fn save_png_from_channels(
    path: &str,
    data: &[u8],
    channels: usize,
    dimensions: (u32, u32)
) -> Result<(), String> {
    // Convert the raw channel data to RGBA format for saving
    let rgba_data = match channels {
        1 => data.iter().flat_map(|&v| vec![v, v, v, 255]).collect(),
        4 => data.to_vec(),
        _ => return Err("Unsupported number of channels".to_string())
    };

    // Save the RGBA data as a PNG file using png crate
    let file =
        std::fs::File::create(path).map_err(|e| format!("Failed to create file {path}: {e}"))?;
    let w = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, dimensions.0, dimensions.1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("Failed to write PNG header for {path}: {e}"))?;
    writer
        .write_image_data(&rgba_data)
        .map_err(|e| format!("Failed to write PNG data for {path}: {e}"))
}
