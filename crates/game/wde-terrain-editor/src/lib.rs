use wde_egui::prelude::*;
use wde_terrain::prelude::*;
use wde_logger::prelude::*;
use bevy::prelude::*;

use crate::{paint::{brush::{PaintMode, PaintBrush}, paint_manager::{PaintManager, PaintManagerPlugin}}, processor::PaintProcessorPlugin};

mod paint;
mod processor;

pub mod prelude {
    pub use super::TerrainEditorPlugin;
}

pub struct TerrainEditorPlugin;
impl Plugin for TerrainEditorPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<SaveManager>()
            .add_plugins(PaintManagerPlugin)
            .add_plugins(PaintProcessorPlugin)
            .add_systems(Startup, init)
            .add_systems(Update, egui_paint_debug)
            .add_systems(Update, save_extracted_tiles)
            .add_message::<ExtractedTileMessage>();
    }
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn a terrain
    commands.spawn((
        Terrain::load("tests/terrain"),
        TerrainRenderer::new(&asset_server),
        TerrainPhysics::default()
    ));

    // Spawn a default brush for testing
    commands.spawn(PaintBrush::default());
}


#[derive(Resource, Default)]
struct SaveManager {
    // True if we are currently saving the terrain
    saving: bool,
    // List of tiles to save, with their position and map type (0 for heightmap, 1 for splatmap)
    tiles_to_save: Vec<(TilePos, u32, u32)>
}

fn egui_paint_debug(
    ctx: Res<EguiContext>,
    mut paint_manager: ResMut<PaintManager>,
    mut query: Query<&mut PaintBrush>,
    terrain: Query<&Terrain>,
    mut extractor: ResMut<TerrainExtractor>,
    mut save_manager: ResMut<SaveManager>
) {
    egui::Window::new("Paint Debug")
        .default_pos([40.0, 20.0])
        .show(&ctx.0, |ui| {
            ui.heading("Current Brush");
            if let Ok(mut brush) = query.single_mut() {
                ui.add(egui::Slider::new(&mut brush.radius, 0.0..=100.0).text("Radius"));
                ui.add(egui::Slider::new(&mut brush.strength, 0.0..=1.0).text("Strength"));
                if brush.paint_mode == PaintMode::Paint || brush.paint_mode == PaintMode::Erase {
                    ui.label("Color:");
                    ui.color_edit_button_rgba_unmultiplied(&mut brush.color);
                }
                ui.horizontal(|ui| {
                    ui.label("Type:");
                    egui::ComboBox::from_id_salt("brush_type")
                        .selected_text(format!("{:?}", brush.paint_mode))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut brush.paint_mode, PaintMode::Paint, "Paint");
                            ui.selectable_value(&mut brush.paint_mode, PaintMode::Erase, "Erase");
                            ui.selectable_value(&mut brush.paint_mode, PaintMode::Raise, "Raise");
                            ui.selectable_value(&mut brush.paint_mode, PaintMode::Lower, "Lower");
                            ui.selectable_value(&mut brush.paint_mode, PaintMode::Smooth, "Smooth");
                            ui.selectable_value(&mut brush.paint_mode, PaintMode::Flatten, "Flatten");
                        });
                });
            } else {
                ui.label("No brush found");
            }

            ui.separator();
            ui.heading("Paint Manager");
            ui.checkbox(&mut paint_manager.active, "Active");
            ui.label(format!("Painting: {}", paint_manager.painting));
            ui.label(format!("Should Flush: {}", paint_manager.should_flush));
            ui.label(format!("Commands: {}", paint_manager.commands.as_ref().map_or(0, |c| c.len())));

            ui.separator();
            ui.heading("Save");
            if ui.button("Save Terrain").clicked() {
                info!("Saving terrain to disk...");
                if save_manager.saving {
                    ui.label("Already saving...");
                    return;
                }
                let terrain = match terrain.single() {
                    Ok(terrain) => terrain,
                    Err(_) => return,
                };
                for pos in terrain.pos_to_tile.keys() {
                    for t in 0..2 {
                        extractor.queue_tile_extraction(*pos, t, 0);
                        save_manager.tiles_to_save.push((*pos, t, 0));
                    }
                }
                save_manager.saving = true;
            }
        });
}

fn save_extracted_tiles(
    terrain: Query<&Terrain>,
    mut save_manager: ResMut<SaveManager>,
    mut extracted_tiles: MessageReader<ExtractedTileMessage>,
) {
    // If we are not currently saving, ignore the extracted tiles
    if !save_manager.saving {
        return;
    }

    // Get the terrain from main world
    let terrain = match terrain.single() {
        Ok(terrain) => terrain,
        Err(_) => return,
    };

    // Process each extracted tile message
    for message in extracted_tiles.read() {
        let ExtractedTileMessage { pos, map_type, splat_map_index, data } = message;

        // Save the extracted tile data to a file
        let cur_dir = std::env::current_dir().unwrap();
        let full_path = format!("{}/res/{}", cur_dir.display(), terrain.path);
        match map_type {
            0 => {
                let path = format!("{}/heightmap_{}_{}.png", full_path, pos.x, pos.y);
                if let Err(e) = save_png_from_channels(&path, data, 1, (RENDER_TILE_SUBDIVISIONS, RENDER_TILE_SUBDIVISIONS)) {
                    error!("Failed to save heightmap for tile ({}, {}): {}", pos.x, pos.y, e);
                }
            },
            1 => {
                let path = format!("{}/splatmap_{}_{}-{}.png", full_path, pos.x, pos.y, splat_map_index);
                if let Err(e) = save_png_from_channels(&path, data, 4, (RENDER_TILE_SUBDIVISIONS, RENDER_TILE_SUBDIVISIONS)) {
                    error!("Failed to save splatmap for tile ({}, {}): {}", pos.x, pos.y, e);
                }
            },
            _ => continue,
        };
        save_manager.tiles_to_save.retain(|(p, t, s)| !(p == pos && t == map_type && s == splat_map_index));
    }

    // If all tiles have been saved, mark saving as false
    if save_manager.tiles_to_save.is_empty() {
        info!("Finished saving terrain.");
        save_manager.saving = false;
    }
}


fn save_png_from_channels(path: &str, data: &[u8], channels: usize, dimensions: (u32, u32)) -> Result<(), String> {
    // Convert the raw channel data to RGBA format for saving
    let rgba_data = match channels {
        1 => data.iter().flat_map(|&v| vec![v, v, v, 255]).collect(),
        4 => data.to_vec(),
        _ => return Err("Unsupported number of channels".to_string()),
    };

    // Save the RGBA data as a PNG file using png crate
    let file = std::fs::File::create(path).map_err(|e| format!("Failed to create file {path}: {e}"))?;
    let w = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, dimensions.0, dimensions.1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| format!("Failed to write PNG header for {path}: {e}"))?;
    writer.write_image_data(&rgba_data).map_err(|e| format!("Failed to write PNG data for {path}: {e}"))
}
