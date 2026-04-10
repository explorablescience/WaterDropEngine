use bevy::prelude::*;
use wde_editor::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

use crate::{
    core::placement_config::PlacementConfig,
    editor::{PlacementTool, PlacementUI}
};

pub fn show_ui(
    mut commands: Commands,
    ctx: Res<UIContext>,
    mut ui_menu: ResMut<UIMenu>,
    mut placement_ui: ResMut<PlacementUI>,
    placement_config: Res<PlacementConfig>
) {
    // Reset tool if disabled
    if !placement_ui.enabled {
        for tool in PlacementTool::iter() {
            reset_tool(&mut commands, tool, &mut placement_ui);
        }
    }

    // Show UI
    UIWindow::new("Placement Debug")
        .open(ui_menu.clicked_mut("Terrain/Placement"))
        .show(&ctx.0, |ui| {
            ui.checkbox(&mut placement_ui.enabled, "Enabled");

            // If not enabled, return early
            if !placement_ui.enabled {
                return;
            }

            ui.label("Tool:");
            ui.selectable_value(
                &mut placement_ui.selected_tool,
                PlacementTool::Place,
                "Place"
            );

            ui.separator();

            // Show placement entity UI
            if placement_ui.selected_tool == PlacementTool::Place {
                // Select entity to place
                ui.label("Placement Entity:");
                let old_label = placement_ui.placement_entry_label.clone();
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut placement_ui.placement_entry_label, None, "None");
                    for label in placement_config.labels.iter() {
                        ui.selectable_value(
                            &mut placement_ui.placement_entry_label,
                            Some(label.clone()),
                            label
                        );
                    }
                });

                // Update placement entry based on selected label
                if old_label != placement_ui.placement_entry_label {
                    if let Some(label) = &placement_ui.placement_entry_label {
                        if let Some(entry) =
                            placement_config.entries.iter().find(|e| &e.label == label)
                        {
                            placement_ui.placement_entry = Some(entry.clone());
                            let entity = placement_ui
                                .placement_entity
                                .unwrap_or_else(|| commands.spawn_empty().id());
                            placement_ui.placement_entity = Some(entity);
                            let mut entity = match commands.get_entity(entity) {
                                Ok(entity) => entity,
                                Err(_) => commands.entity(entity)
                            };
                            entity.insert((
                                Transform::default()
                                    .with_translation(Vec3::new(10000.0, -10000.0, 10000.0)),
                                Mesh3d(entry.asset.models[0].0.clone()),
                                PbrMaterial3d(entry.asset.models[0].1.clone()),
                            ));
                        } else {
                            reset_tool(
                                &mut commands,
                                placement_ui.selected_tool,
                                &mut placement_ui
                            );
                        }
                    } else {
                        reset_tool(&mut commands, placement_ui.selected_tool, &mut placement_ui);
                    }
                }

                // Display selected entity info
                if let Some(entry) = &placement_ui.placement_entry {
                    ui.label(format!("Selected Entity: {}", entry.label));
                    ui.label(format!("Model: {}", entry.asset.path));
                    ui.label(format!("Extent: {}x{}", entry.extent.x, entry.extent.y));
                }
            } else {
                reset_tool(&mut commands, placement_ui.selected_tool, &mut placement_ui);
            }
        });
}

fn reset_tool(commands: &mut Commands, tool: PlacementTool, placement_ui: &mut PlacementUI) {
    if tool == PlacementTool::Place {
        placement_ui.placement_entry_label = None;
        placement_ui.placement_entry = None;
        if let Some(entity) = placement_ui.placement_entity {
            commands.entity(entity).despawn();
            placement_ui.placement_entity = None;
        }
    }
}
