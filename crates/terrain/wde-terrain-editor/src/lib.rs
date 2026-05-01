//! Terrain editor plugin for WaterDropEngine.
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
        .add_systems(PreStartup, init)
        .add_systems(Update, handle_extracted_tiles)
        .add_message::<ExtractedTileMessage>();
    }
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    let entity = commands
        .spawn((
            Name::new("Terrain"),
            Transform::default(),
            Terrain::load("tests/terrain"),
            TerrainRenderer::new(&asset_server),
            TerrainPhysics::default()
        ))
        .id();

    commands.spawn((
        Name::new("Terrain Default Brush"),
        PaintBrush::default(),
        ChildOf(entity)
    ));
}

#[derive(Resource, Default)]
struct SaveManager {
    saving: bool,
    tiles_to_save: Vec<(ChunkPos, u32, u32)>
}

/// Handle all tile readbacks from the GPU:
/// - Always update physics when a heightmap arrives.
/// - When in save mode, also write the tile to disk.
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

        // Always feed heightmaps back into the physics system.
        if *map_type == 0 {
            terrain.dirty_physics.push((*pos, 0, 0, data.clone()));
        }

        // Optionally save to disk.
        if save_manager.saving {
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
                        error!("Failed to save heightmap ({}, {}): {}", pos.x, pos.y, e);
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
                        error!("Failed to save splatmap ({}, {}): {}", pos.x, pos.y, e);
                    }
                }
                _ => continue
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

fn save_png_from_channels(
    path: &str,
    data: &[u8],
    channels: usize,
    dimensions: (u32, u32)
) -> Result<(), String> {
    let rgba_data: Vec<u8> = match channels {
        1 => data.iter().flat_map(|&v| [v, v, v, 255]).collect(),
        4 => data.to_vec(),
        _ => return Err("Unsupported number of channels".to_string())
    };

    let file =
        std::fs::File::create(path).map_err(|e| format!("Failed to create file {path}: {e}"))?;
    let w = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, dimensions.0, dimensions.1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("Failed to write PNG header: {e}"))?;
    writer
        .write_image_data(&rgba_data)
        .map_err(|e| format!("Failed to write PNG data: {e}"))
}
