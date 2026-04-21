use bevy::prelude::*;
use wde_editor::prelude::*;
use wde_gltf::prelude::*;

use crate::{
    core::placement_config::PlacementConfig,
    placement::{
        TerrainPlacementManager, TerrainPlacementMode, move_and_delete::MoveAndDeleteManager,
        place::ui_place_entity
    }
};

#[allow(clippy::too_many_arguments)]
pub fn show_ui(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    ctx: Res<UIContext>,
    mut ui_menu: ResMut<UIMenu>,
    mut manager: ResMut<TerrainPlacementManager>,
    config: Res<PlacementConfig>,
    gltf_models: Res<Assets<GltfAsset>>,
    mut move_and_delete_manager: ResMut<MoveAndDeleteManager>
) {
    UIWindow::new("Terrain Placement")
        .open(ui_menu.clicked_mut("Terrain/Placement"))
        .show(&ctx.0, |ui| {
            ui.label("Mode:");
            let old_mode = manager.mode.clone();
            ui.horizontal(|ui| {
                ui.selectable_value(&mut manager.mode, TerrainPlacementMode::None, "None");
                ui.selectable_value(&mut manager.mode, TerrainPlacementMode::Place, "Place");
                ui.selectable_value(&mut manager.mode, TerrainPlacementMode::Move, "Move");
                ui.selectable_value(&mut manager.mode, TerrainPlacementMode::Remove, "Remove");
            });

            // Change mode if it was updated in the UI
            if old_mode != manager.mode {
                reset_tool(&mut commands, &mut manager, &mut move_and_delete_manager);
            }

            // Show placement entity UI
            ui.separator();
            match manager.mode {
                TerrainPlacementMode::Place => {
                    ui_place_entity(
                        commands,
                        ui,
                        &mut manager,
                        &config,
                        &gltf_models,
                        &asset_server
                    );
                }
                TerrainPlacementMode::Move => {}   // TODO
                TerrainPlacementMode::Remove => {} // TODO
                TerrainPlacementMode::None => {
                    reset_tool(&mut commands, &mut manager, &mut move_and_delete_manager);
                }
            }
        });
}

fn reset_tool(
    commands: &mut Commands,
    manager: &mut TerrainPlacementManager,
    move_and_delete_manager: &mut MoveAndDeleteManager
) {
    manager.place_selected_entry_label = None;
    manager.place_selected_entry = None;
    commands
        .entity(manager.entity)
        .insert(Transform::from_translation(Vec3::MIN));
    commands.entity(manager.entity).despawn_children();
    move_and_delete_manager.should_reset_on_next_update = true;
}
