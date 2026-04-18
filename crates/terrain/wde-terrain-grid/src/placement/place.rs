use wde_logger::prelude::*;

use bevy::{prelude::*, window::PrimaryWindow};
use wde_camera::prelude::*;
use wde_editor::prelude::*;
use wde_pbr::prelude::*;
use wde_physics::prelude::*;
use wde_renderer::prelude::*;
use wde_gltf::prelude::*;

use crate::{
    core::placement_config::PlacementConfig,
    placement::{TerrainPlacementManager, TerrainPlacementMode, ui::reset_tool},
    prelude::{GridEntity, GridRotation}
};

pub fn ui_place_entity(
    mut commands: Commands,
    ui: &mut ui::egui::Ui,
    manager: &mut TerrainPlacementManager,
    config: &PlacementConfig,
    gltf_models: &Assets<GltfAsset>,
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
        reset_tool(&mut commands, manager);
        return;
    }

    // Update placement entry based on selected label
    if old_label != manager.place_selected_entry_label {
        if let Some(entry) = config
            .entries
            .iter()
            .find(|e| &e.label == manager.place_selected_entry_label.as_ref().unwrap())
        {
            manager.place_selected_entry = Some(entry.clone());

            // Get the model to place
            let model = match gltf_models.get(&entry.asset) {
                Some(model) => model,
                None => return, // Model not loaded yet, will try again in the next frame
            };

            // Add the elements as children of the placement entity
            let children = model
                .models
                .iter()
                .map(|(mesh, material)| {
                    commands
                        .spawn((
                            Name::new(format!("Placement Entity - {}", entry.label)),
                            Transform::default(),
                            Mesh3d(mesh.clone()),
                            PbrMaterial3d(material.clone())
                        ))
                        .id()
                })
                .collect::<Vec<Entity>>();
            commands.entity(manager.entity).add_children(&children);
        } else {
            error!(
                "Selected placement entry label '{}' not found in config entries",
                manager.place_selected_entry_label.as_ref().unwrap()
            );
            reset_tool(&mut commands, manager);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn place_update(
    mut commands: Commands,
    manager: Res<TerrainPlacementManager>,
    phworld: Res<PhysicsWorld>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&GlobalTransform, &CameraView), With<Camera>>,
    mut local_rot: Local<GridRotation>,
    mouse_input: Res<ButtonInput<MouseButton>>
) {
    if manager.mode != TerrainPlacementMode::Place || manager.place_selected_entry.is_none() {
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

    // Create the ray from ndc position
    let cursor_ndc_position = match window.cursor_position() {
        Some(pos) => pos / window.size(),
        None => return
    };
    let (camera_transform, camera_view) = camera_query
        .single()
        .map_err(|_| "No camera found")
        .unwrap();
    let ray = Ray::from_ndc(
        cursor_ndc_position,
        window.size().x / window.size().y,
        camera_transform,
        camera_view
    );

    // Cast the ray in the physics world
    if let Some((_, toi)) = phworld.as_ref().cast_ray(&ray, &RayCastConfig::default()) {
        let hit_point = ray.point_at(toi);

        // Move the placement entity to the hit point with the correct rotation
        let grid_entity = GridEntity::new(
            Vec2::new(hit_point.x, hit_point.z),
            manager.place_selected_entry.as_ref().unwrap().extent,
            *local_rot
        );
        commands.entity(manager.entity).insert(
            Transform::from_rotation(Quat::from_rotation_y(local_rot.rotation())).with_translation(
                Vec3::new(grid_entity.center().x, 0.0, grid_entity.center().y)
            )
        );
    }
}
