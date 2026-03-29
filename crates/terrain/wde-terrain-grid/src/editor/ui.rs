use wde_editor::prelude::*;
use bevy::prelude::*;

use crate::editor::{PlacementUI, PlacementTool};

pub fn init_ui(mut ui_menu: ResMut<UIMenu>) {
    ui_menu.push("Terrain/Placement");
}

pub fn show_ui(ctx: Res<UIContext>, mut ui_menu: ResMut<UIMenu>, mut placement_ui: ResMut<PlacementUI>) {
    if !ui_menu.is_clicked("Terrain/Placement") {
        return;
    }

    UIWindow::new("Placement Debug")
        .open(ui_menu.clicked_mut("Terrain/Placement").unwrap())
        .show(&ctx.0, |ui| {
            ui.checkbox(&mut placement_ui.enabled, "Enabled");
            ui.separator();
            ui.label("Selected Tool:");
            ui.selectable_value(&mut placement_ui.selected_tools, PlacementTool::Place, "Place");
            ui.separator();
            ui.checkbox(&mut placement_ui.placement_show_entity, "Show Placement Preview");
            ui.label("Placement Extent:");
            ui.horizontal(|ui| {
                ui.add(DragValue::new(&mut placement_ui.placement_extent.x).range(1..=10).prefix("x: "));
                ui.add(DragValue::new(&mut placement_ui.placement_extent.y).range(1..=10).prefix("y: "));
            });
        });
}
