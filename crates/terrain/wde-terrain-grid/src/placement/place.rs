use wde_logger::prelude::*;
use wde_terrain::TERRAIN_BUILDINGS_COLLIDER_GROUP;

use crate::{core::placement_config::PlacementConfigEntry, placement::TerrainCursorPos};
use bevy::prelude::*;
use wde_editor::prelude::*;
use wde_gltf::prelude::*;
use wde_pbr::prelude::*;
use wde_physics::prelude::*;
use wde_renderer::prelude::{Color, *};

use crate::{
    core::placement_config::PlacementConfig,
    placement::{TerrainPlacementManager, TerrainPlacementMode},
    prelude::{Grid, GridEntity, GridRotation},
    render::ghost::ghost_material::GhostMaterial
};

#[derive(Component)]
pub(crate) struct GhostMaterialStorer {
    pbr_material: Handle<PbrMaterial>,
    ok_material: Handle<GhostMaterial>,
    nok_material: Handle<GhostMaterial>,
    current_validity: bool
}

pub fn ui_place_entity(
    mut commands: Commands,
    ui: &mut ui::egui::Ui,
    manager: &mut TerrainPlacementManager,
    config: &PlacementConfig,
    gltf_models: &Assets<GltfAsset>,
    asset_server: &AssetServer
) {
    // Select entity to place
    ui.label("Placement Entity:");
    let old_label = manager.place_selected_entry_label.clone();
    ui.horizontal(|ui| {
        ui.selectable_value(&mut manager.place_selected_entry_label, None, "None");
        for entry in config.entries.iter() {
            ui.selectable_value(
                &mut manager.place_selected_entry_label,
                Some(entry.label.clone()),
                entry.label.clone()
            );
        }
    });
    if manager.place_selected_entry_label.is_none() {
        reset_placement_tool(&mut commands, manager);
        return;
    }

    // Update placement entry based on selected label
    if old_label != manager.place_selected_entry_label {
        if let Some(entry) = config
            .entries
            .iter()
            .find(|e| &e.label == manager.place_selected_entry_label.as_ref().unwrap())
        {
            spawn_new_ghost_entity(&mut commands, entry, gltf_models, asset_server, manager);
        } else {
            error!(
                "Selected placement entry label '{}' not found in config entries",
                manager.place_selected_entry_label.as_ref().unwrap()
            );
            reset_placement_tool(&mut commands, manager);
        }
    }
}

pub fn handle_new_entity_to_place(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut manager: ResMut<TerrainPlacementManager>,
    gltf_models: Res<Assets<GltfAsset>>
) {
    let Some(ref entry) = manager.new_entry_to_place else {
        return;
    };
    let label = entry.label.clone();
    spawn_new_ghost_entity(
        &mut commands,
        &entry.clone(),
        &gltf_models,
        &asset_server,
        &mut manager
    );
    manager.place_selected_entry_label = Some(label);
    manager.new_entry_to_place = None;
}

fn spawn_new_ghost_entity(
    commands: &mut Commands,
    entry: &PlacementConfigEntry,
    gltf_models: &Assets<GltfAsset>,
    asset_server: &AssetServer,
    manager: &mut TerrainPlacementManager
) {
    manager.place_selected_entry = Some(entry.clone());

    // Get the model to place
    let model = match gltf_models.get(&entry.asset) {
        Some(model) => model,
        None => return // Model not loaded yet, will try again in the next frame
    };

    // Add the elements as children of the placement entity
    let children = model
        .models
        .iter()
        .map(|(mesh, material)| {
            // Create material
            let ok_material = asset_server.add(GhostMaterial {
                overlay: Color::LinearRgba(0.8, 0.8, 0.8, 0.1),
                desaturate: 0.8,
                rim_power: 3.0,
                rim_strength: 0.01,
                pbr_material: Some(material.clone()),
                ..Default::default()
            });
            let nok_material = asset_server.add(GhostMaterial {
                overlay: Color::LinearRgba(1.0, 0.0, 0.0, 0.16),
                desaturate: 0.6,
                rim_power: 3.0,
                rim_strength: 0.01,
                pbr_material: Some(material.clone()),
                ..Default::default()
            });

            // Spawn entity
            commands
                .spawn((
                    Name::new(format!("Placement Entity - {}", entry.label)),
                    Transform::default(),
                    Mesh3d(mesh.clone()),
                    PbrMaterial3d(ok_material.clone()),
                    GhostMaterialStorer {
                        pbr_material: material.clone(),
                        ok_material,
                        nok_material,
                        current_validity: true
                    }
                ))
                .id()
        })
        .collect::<Vec<Entity>>();
    commands.entity(manager.entity).add_children(&children);
}

#[allow(clippy::too_many_arguments)]
pub fn place_update(
    terrain_cursor_pos: Res<TerrainCursorPos>,
    mut commands: Commands,
    mut manager: ResMut<TerrainPlacementManager>,
    mut local_rot: Local<GridRotation>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut grid: ResMut<Grid>,
    children_query: Query<&Children>,
    mut material_query: Query<(&Mesh3d, &mut GhostMaterialStorer)>,
    gltf_models: Res<Assets<GltfAsset>>
) {
    if manager.place_selected_entry.is_none()
        || (manager.mode != TerrainPlacementMode::Place
            && manager.mode != TerrainPlacementMode::Move)
    {
        return;
    }

    // Toggle rotation on right click
    if mouse_input.just_pressed(MouseButton::Middle) {
        *local_rot = match *local_rot {
            GridRotation::R0 => GridRotation::R90,
            GridRotation::R90 => GridRotation::R180,
            GridRotation::R180 => GridRotation::R270,
            GridRotation::R270 => GridRotation::R0
        };
    }

    // Cast the ray in the physics world
    if let Some(pos) = terrain_cursor_pos.pos() {
        // Move the placement entity to the hit point with the correct rotation
        let grid_entity = GridEntity::new(
            Vec2::new(pos.x, pos.z),
            manager.place_selected_entry.as_ref().unwrap().extent,
            *local_rot
        );
        commands.entity(manager.entity).insert(
            Transform::from_rotation(Quat::from_rotation_y(local_rot.rotation())).with_translation(
                Vec3::new(grid_entity.center().x, 0.0, grid_entity.center().y)
            )
        );

        // Check for placement validity and update ghost material accordingly
        let is_valid = grid.is_area_free(&grid_entity);
        for child in children_query.get(manager.entity).unwrap().iter() {
            if let Ok((_, mut material_storer)) = material_query.get_mut(child) {
                if material_storer.current_validity == is_valid {
                    continue; // No change in validity, skip
                }
                let new_material = if is_valid {
                    material_storer.ok_material.clone()
                } else {
                    material_storer.nok_material.clone()
                };
                commands.entity(child).insert(PbrMaterial3d(new_material));
                material_storer.current_validity = is_valid;
            }
        }

        // Place the entity on left click if the position is valid
        if mouse_input.just_pressed(MouseButton::Left) && is_valid {
            let grid_entity = GridEntity::new(
                Vec2::new(pos.x, pos.z),
                manager.place_selected_entry.as_ref().unwrap().extent,
                *local_rot
            );

            // Spawn parent
            let parent = commands
                .spawn((
                    Name::new(
                        manager
                            .place_selected_entry_label
                            .as_ref()
                            .unwrap()
                            .to_string()
                    ),
                    Transform::from_rotation(Quat::from_rotation_y(local_rot.rotation()))
                        .with_translation(Vec3::new(
                            grid_entity.center().x,
                            0.0,
                            grid_entity.center().y
                        )),
                    grid_entity.clone(),
                    ChildOf(grid.get_parent().unwrap())
                ))
                .id();

            // Spawn collider child
            let bbox = match gltf_models.get(&manager.place_selected_entry.as_ref().unwrap().asset)
            {
                Some(model) => &model.bbox,
                None => return // Model not loaded yet, should not happen since we check this in the UI code, but just in case
            };
            let bbox_extent = bbox.max - bbox.min;
            let bbox_offset = Vec3::new(
                bbox.min.x + bbox_extent.x / 2.0,
                bbox.min.y + bbox_extent.y / 2.0,
                bbox.min.z + bbox_extent.z / 2.0
            );
            commands.spawn((
                Name::new("Collider"),
                Transform::from_translation(bbox_offset),
                Collider::from(
                    BoxCollider::new(bbox_extent).with_group(TERRAIN_BUILDINGS_COLLIDER_GROUP)
                ),
                ChildOf(parent)
            ));

            // Spawn mesh children
            for child in children_query.get(manager.entity).unwrap().iter() {
                commands.spawn((
                    Name::new(format!(
                        "{} Child",
                        manager.place_selected_entry_label.as_ref().unwrap()
                    )),
                    Transform::default(),
                    grid_entity.clone(),
                    Mesh3d(match material_query.get(child) {
                        Ok((mesh, _)) => mesh.0.clone(),
                        Err(_) => continue
                    }),
                    PbrMaterial3d(match material_query.get(child) {
                        Ok((_, material_storer)) => material_storer.pbr_material.clone(),
                        Err(_) => continue
                    }),
                    ChildOf(parent)
                ));
            }

            // Mark the grid area as occupied
            grid.set_entity(&grid_entity, parent);

            // Reset placement tool
            reset_placement_tool(&mut commands, &mut manager);
        }
    }
}

fn reset_placement_tool(commands: &mut Commands, manager: &mut TerrainPlacementManager) {
    manager.place_selected_entry_label = None;
    manager.place_selected_entry = None;
    commands
        .entity(manager.entity)
        .insert(Transform::from_translation(Vec3::MIN));
    commands.entity(manager.entity).despawn_children();
}
