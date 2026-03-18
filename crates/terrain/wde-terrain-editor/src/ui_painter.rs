use wde_logger::prelude::*;
use wde_editor::prelude::*;
use wde_terrain::prelude::*;
use bevy::prelude::*;

use crate::{SaveManager, paint::{brush::{PaintBrush, PaintMode}, paint_manager::PaintManager}};

pub struct TerrainEditorUIPlugin;
impl Plugin for TerrainEditorUIPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, init_ui)
            .add_systems(Update, ui_paint_terrain)
            .add_systems(Update, ui_save_terrain);
    }
}

fn init_ui(mut ui_menu: ResMut<UIMenu>) {
    ui_menu.push("Terrain/Paint");
    ui_menu.push("Terrain/Save");
}

fn ui_paint_terrain(
    ctx: Res<UIContext>,
    mut paint_manager: ResMut<PaintManager>,
    mut query: Query<&mut PaintBrush>,
    ui_menu: Res<UIMenu>
) {
    // Check if the menu item was clicked
    if !ui_menu.is_clicked("Terrain/Paint") {
        return;
    }

    // Draw UI
    UIWindow::new("Paint Debug")
        .default_pos([40.0, 20.0])
        .show(&ctx.0, |ui| {
            ui.heading("Current Brush");
            if let Ok(mut brush) = query.single_mut() {
                ui.add(Slider::new(&mut brush.radius, 0.0..=100.0).text("Radius"));
                ui.add(Slider::new(&mut brush.strength, 0.0..=1.0).text("Strength"));
                if brush.paint_mode == PaintMode::Paint || brush.paint_mode == PaintMode::Erase {
                    ui.label("Color:");
                    ui.color_edit_button_rgba_unmultiplied(&mut brush.color);
                }
                ui.horizontal(|ui| {
                    ui.label("Type:");
                    ComboBox::from_id_salt("brush_type")
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
        });
}

fn ui_save_terrain(
    ctx: Res<UIContext>,
    terrain: Query<&Terrain>,
    mut extractor: ResMut<TerrainExtractor>,
    mut save_manager: ResMut<SaveManager>,
    ui_menu: Res<UIMenu>
) {
    // Check if the menu item was clicked
    if !ui_menu.is_clicked("Terrain/Save") {
        return;
    }

    // Clear the previous list of tiles to extract
    extractor.clear_tiles_to_extract();
    

    // Draw UI
    UIWindow::new("Save Terrain")
        .default_pos([40.0, 200.0])
        .show(&ctx.0, |ui| {
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