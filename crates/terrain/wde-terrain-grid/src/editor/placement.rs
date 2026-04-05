use bevy::{prelude::*, window::PrimaryWindow};
use wde_camera::prelude::*;
use wde_physics::prelude::*;

use crate::{
    editor::{PlacementTool, PlacementUI},
    prelude::{Grid, GridEntity, GridRotation}
};

#[allow(clippy::too_many_arguments)]
pub fn handle_placement_tool(
    mut commands: Commands,
    placement_ui: Res<PlacementUI>,
    phworld: Res<PhysicsWorld>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Transform, &CameraView), With<Camera>>,
    mut grid: ResMut<Grid>,
    mut local_rot: Local<GridRotation>,
    mouse_input: Res<ButtonInput<MouseButton>>
) {
    if !placement_ui.enabled
        || placement_ui.placement_entity.is_none()
        || placement_ui.selected_tool != PlacementTool::Place
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
        let hit_point = Vec2::new(hit_point.x, hit_point.z);

        // Clear the grid of placeholder entity (used to store the position of the entity being placed)
        grid.remove_entity(Entity::PLACEHOLDER);

        // Update the grid with the new position of the entity
        let grid_entity = GridEntity::new(
            hit_point,
            placement_ui.placement_entry.as_ref().unwrap().extent,
            *local_rot
        );
        commands
            .entity(placement_ui.placement_entity.unwrap())
            .insert(
                Transform::from_rotation(Quat::from_rotation_y(local_rot.rotation()))
                    .with_translation(Vec3::new(
                        grid_entity.center().x,
                        0.0,
                        grid_entity.center().y
                    ))
            );
        let occupied_tiles = grid_entity.footprint();
        for (chunk_pos, local_pos) in occupied_tiles {
            grid.set_entity_at(*chunk_pos, *local_pos, Entity::PLACEHOLDER);
        }
    }
}
