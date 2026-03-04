use wde_egui::prelude::*;
use wde_terrain::prelude::*;
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
            .add_plugins(PaintManagerPlugin)
            .add_plugins(PaintProcessorPlugin)
            .add_systems(Startup, init)
            .add_systems(Update, egui_paint_debug);
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

fn egui_paint_debug(
    ctx: Res<EguiContext>,
    mut paint_manager: ResMut<PaintManager>,
    mut query: Query<&mut PaintBrush>
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
        });
}

