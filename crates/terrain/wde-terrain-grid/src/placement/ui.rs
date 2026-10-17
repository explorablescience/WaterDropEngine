use bevy::prelude::*;
use wde_editor::prelude::{ui::egui, *};
use wde_gltf::prelude::*;

use crate::{
    core::entries::PlacementConfig,
    placement::{
        BuildingCutaway, TerrainPlacementManager, TerrainPlacementMode,
        move_and_delete::MoveAndDeleteManager, place::ui_place_entity
    }
};

const MODES: [(TerrainPlacementMode, &str, &str); 3] = [
    (
        TerrainPlacementMode::Place,
        "➕ Build",
        "Pick a building, then place it on the grid."
    ),
    (
        TerrainPlacementMode::Move,
        "↔ Move",
        "Click a placed building to pick it up again."
    ),
    (
        TerrainPlacementMode::Remove,
        "🗑 Remove",
        "Click a placed building to demolish it."
    )
];

#[allow(clippy::too_many_arguments)]
pub fn show_ui(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    ctx: Res<UIContext>,
    mut ui_menu: ResMut<UIMenu>,
    mut manager: ResMut<TerrainPlacementManager>,
    config: Res<PlacementConfig>,
    gltf_models: Res<Assets<GltfAsset>>,
    mut move_and_delete_manager: ResMut<MoveAndDeleteManager>,
    mut cutaway: ResMut<BuildingCutaway>,
    ui_textures: Option<Res<UITextures>>
) {
    // Tools are only active while the window is open, starting in build mode
    let open = ui_menu.clicked_mut("Terrain/Placement");
    if !*open {
        if manager.mode != TerrainPlacementMode::None {
            manager.mode = TerrainPlacementMode::None;
            reset_tool(&mut commands, &mut manager, &mut move_and_delete_manager);
        }
        return;
    }
    if manager.mode == TerrainPlacementMode::None {
        manager.mode = TerrainPlacementMode::Place;
    }

    UIWindow::new("Terrain Placement")
        .open(open)
        .default_width(470.0)
        .resizable(true)
        .show(&ctx.0, |ui| {
            // Mode bar and view toggle
            let old_mode = manager.mode.clone();
            ui.horizontal(|ui| {
                for (mode, label, hint) in MODES {
                    let selected = manager.mode == mode;
                    let button = egui::Button::new(label)
                        .selected(selected)
                        .min_size(egui::vec2(72.0, 26.0));
                    if ui.add(button).on_hover_text(hint).clicked() {
                        manager.mode = mode;
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.checkbox(&mut cutaway.0, "Cutaway")
                        .on_hover_text("Show buildings without their roof (V).");
                });
            });
            if old_mode != manager.mode {
                reset_tool(&mut commands, &mut manager, &mut move_and_delete_manager);
            }
            ui.separator();

            match manager.mode {
                TerrainPlacementMode::Place => {
                    ui_place_entity(
                        commands,
                        ui,
                        &mut manager,
                        &config,
                        &gltf_models,
                        &asset_server,
                        ui_textures.as_deref()
                    );
                }
                TerrainPlacementMode::Move | TerrainPlacementMode::Remove => {
                    let hint = MODES.iter().find(|m| m.0 == manager.mode).unwrap().2;
                    ui.label(egui::RichText::new(hint).weak());
                }
                TerrainPlacementMode::None => {}
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
    manager.ghost_connections = None;
    manager.line_network = None;
    commands
        .entity(manager.entity)
        .insert(Transform::from_translation(Vec3::MIN));
    commands.entity(manager.entity).despawn_children();
    move_and_delete_manager.should_reset_on_next_update = true;
}
