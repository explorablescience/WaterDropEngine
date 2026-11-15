use wde_logger::prelude::*;

use bevy::prelude::*;
use wde_editor::prelude::*;
use wde_terrain::{prelude::*, render::dependencies::terrain_buffer::TERRAIN_MATERIAL_COUNT};

use crate::{
    SaveManager,
    paint::{
        brush::{PaintBrush, PaintMode},
        paint_manager::PaintManager
    }
};

pub struct TerrainEditorUIPlugin;
impl Plugin for TerrainEditorUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, ui_paint_terrain)
            .add_systems(Update, ui_save_terrain)
            .add_systems(Update, ui_terrain_settings)
            .add_systems(Update, ui_terrain_debug);
    }
}

fn ui_paint_terrain(
    ctx: Res<UIContext>,
    mut paint_manager: ResMut<PaintManager>,
    mut query: Query<&mut PaintBrush>,
    mut ui_menu: ResMut<UIMenu>
) {
    UIWindow::new("Paint Debug")
        .default_pos([40.0, 20.0])
        .open(ui_menu.clicked_mut("Terrain/Paint"))
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
                            ui.selectable_value(
                                &mut brush.paint_mode,
                                PaintMode::Flatten,
                                "Flatten"
                            );
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
            ui.label(format!(
                "Commands: {}",
                paint_manager.commands.as_ref().map_or(0, |c| c.len())
            ));
        });
}

fn ui_terrain_settings(
    ctx: Res<UIContext>,
    mut settings: ResMut<TerrainRenderSettings>,
    mut ui_menu: ResMut<UIMenu>
) {
    UIWindow::new("Terrain Settings")
        .default_pos([40.0, 400.0])
        .open(ui_menu.clicked_mut("Terrain/Settings"))
        .show(&ctx.0, |ui| {
            ui.heading("Tiling (world size of one texture repeat)");
            for i in 0..TERRAIN_MATERIAL_COUNT {
                ui.add(
                    Slider::new(&mut settings.tiling[i], 0.5..=20.0).text(format!("Material {i}"))
                );
            }
            ui.separator();
            ui.heading("Height blending (relief over the neighbors)");
            for i in 0..TERRAIN_MATERIAL_COUNT {
                ui.add(
                    Slider::new(&mut settings.height_blend[i], 0.0..=2.0)
                        .text(format!("Material {i}"))
                );
            }
        });
}

fn debug_view_label(view: TerrainDebugView, materials: &[TerrainMaterial]) -> String {
    match view {
        TerrainDebugView::Lod => "Levels of Detail".to_string(),
        TerrainDebugView::Materials => "Material Weights".to_string(),
        TerrainDebugView::MaterialWeight(m) => {
            format!("Weight: {}", material_label(m as usize, materials))
        }
        view => format!("{view:?}")
    }
}

fn material_label(m: usize, materials: &[TerrainMaterial]) -> String {
    match materials.get(m) {
        Some(material) => {
            let name = material.path.trim_end_matches('/');
            format!("{m} {}", name.rsplit('/').next().unwrap_or(name))
        }
        None => format!("Material {m}")
    }
}

fn color([r, g, b]: [u8; 3]) -> Color32 {
    Color32::from_rgb(r, g, b)
}

fn ui_terrain_debug(
    ctx: Res<UIContext>,
    mut settings: ResMut<TerrainRenderSettings>,
    terrain: Query<&Terrain>,
    renderer: Query<&TerrainRenderer>,
    cursor: Res<TerrainCursorPos>,
    mut ui_menu: ResMut<UIMenu>
) {
    UIWindow::new("Terrain")
        .default_pos([100.0, 260.0])
        .open(ui_menu.clicked_mut("Debug/Terrain"))
        .show(&ctx.0, |ui| {
            let materials = terrain
                .single()
                .map_or(&[][..], |t| t.config.materials.as_slice());
            let view = settings.debug_view;
            let mut selected = view;
            ui.horizontal(|ui| {
                ui.label("View:");
                ComboBox::from_id_salt("terrain_debug_view")
                    .selected_text(debug_view_label(view, materials))
                    .show_ui(ui, |ui| {
                        let views = [
                            TerrainDebugView::Off,
                            TerrainDebugView::Buildable,
                            TerrainDebugView::Slope,
                            TerrainDebugView::Lod,
                            TerrainDebugView::Chunks,
                            TerrainDebugView::Materials
                        ]
                        .into_iter()
                        .chain((0..materials.len() as u32).map(TerrainDebugView::MaterialWeight));
                        for option in views {
                            ui.selectable_value(
                                &mut selected,
                                option,
                                debug_view_label(option, materials)
                            );
                        }
                    });
            });
            if selected != view {
                settings.debug_view = selected;
            }
            let draw_list = renderer.single().ok().map(|r| &r.draw_list);
            match view {
                TerrainDebugView::Off => {}
                TerrainDebugView::Buildable => {
                    ui.label("Green: buildable. Sand: walkable. Red: too steep to walk.");
                }
                TerrainDebugView::Slope => {
                    ui.label("Blue: flat, to yellow: steepest walkable. Red: too steep to walk.");
                    ui.label("A contour line every 5°.");
                }
                TerrainDebugView::Lod => {
                    ui.label("Chunks by level of detail, with the quads of their mesh.");
                    ui.horizontal(|ui| {
                        ui.label("Chunks drawn by level:");
                        for (level, rgb) in TERRAIN_LOD_DEBUG_COLORS.into_iter().enumerate() {
                            let count = draw_list.map_or(0, |d| d.lod_ranges[level].len());
                            ui.colored_label(color(rgb), format!("{level}: {count}"));
                        }
                    });
                }
                TerrainDebugView::Chunks => {
                    ui.label("White: chunk borders.");
                    ui.colored_label(
                        Color32::from_rgb(255, 0, 255),
                        "Magenta: edges stitched to a coarser neighbor."
                    );
                    if let Some(draw_list) = draw_list {
                        let stitched: usize = draw_list
                            .tiles
                            .iter()
                            .map(|t| t.neighbor_lods.iter().filter(|&&n| n > t.lod).count())
                            .sum();
                        ui.label(format!(
                            "{} chunks drawn, {stitched} stitched edges",
                            draw_list.tiles.len()
                        ));
                    }
                }
                TerrainDebugView::Materials => {
                    ui.label("Splat weights, each material in its color:");
                    for (m, rgb) in TERRAIN_MATERIAL_DEBUG_COLORS
                        .into_iter()
                        .enumerate()
                        .take(materials.len())
                    {
                        ui.colored_label(color(rgb), material_label(m, materials));
                    }
                }
                TerrainDebugView::MaterialWeight(m) => {
                    ui.label("Black: weight 0, to white: weight 1.");
                    ui.colored_label(
                        color(TERRAIN_MATERIAL_DEBUG_COLORS[m as usize % TERRAIN_MATERIAL_COUNT]),
                        "A contour every 0.25."
                    );
                }
            }

            ui.separator();
            let Ok(terrain) = terrain.single() else {
                return;
            };
            ui.label(format!("Terrain: {}", terrain.config.label));
            let Some(pos) = cursor.pos() else {
                ui.label("Cursor: off the terrain");
                return;
            };
            let xz = pos.xz();
            ui.label(format!(
                "Cursor: ({:.1}, {:.1}), height {:.2}",
                xz.x, xz.y, pos.y
            ));
            if let Some(slope) = terrain.slope_at(xz) {
                ui.label(format!(
                    "Slope: {:.1}°, {}, {}",
                    slope.atan().to_degrees(),
                    if terrain.is_walkable(xz) {
                        "walkable"
                    } else {
                        "too steep to walk"
                    },
                    if terrain.is_buildable(xz) {
                        "buildable"
                    } else {
                        "not buildable"
                    }
                ));
            }
        });
}

fn ui_save_terrain(
    ctx: Res<UIContext>,
    terrain: Query<&Terrain>,
    mut extractor: ResMut<TerrainExtractor>,
    mut save_manager: ResMut<SaveManager>,
    mut ui_menu: ResMut<UIMenu>
) {
    UIWindow::new("Save Terrain")
        .default_pos([40.0, 200.0])
        .open(ui_menu.clicked_mut("Terrain/Save"))
        .show(&ctx.0, |ui| {
            ui.heading("Save");
            if ui.button("Save Terrain").clicked() {
                info!("Saving terrain to disk...");
                if save_manager.saving {
                    ui.label("Already saving...");
                    return;
                }
                let terrain = match terrain.single() {
                    Ok(t) => t,
                    Err(_) => return
                };
                for pos in terrain.pos_to_tile.keys() {
                    // Queue heightmap + splatmap for readback.
                    extractor.queue_tile_extraction(*pos, 0, 0);
                    extractor.queue_tile_extraction(*pos, 1, 0);
                    save_manager.tiles_to_save.push((*pos, 0, 0));
                    save_manager.tiles_to_save.push((*pos, 1, 0));
                }
                save_manager.saving = true;
            }
        });
}
