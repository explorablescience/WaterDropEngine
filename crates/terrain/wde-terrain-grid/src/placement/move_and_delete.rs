use wde_logger::prelude::*;

use bevy::{prelude::*, window::PrimaryWindow};
use wde_camera::prelude::*;
use wde_pbr::prelude::*;
use wde_pbr_outline::prelude::*;
use wde_physics::prelude::*;
use wde_renderer::prelude::Color;
use wde_terrain::{TERRAIN_MODULES_COLLIDER_GROUP, prelude::*};

use crate::{
    core::{entries::PlacementConfig, grid::GridEntityEvent},
    placement::{TerrainPlacementManager, TerrainPlacementMode},
    prelude::GridEntity
};

#[derive(Resource)]
pub(crate) struct MoveAndDeleteManager {
    pub move_material: Handle<OutlineMaterial>,
    pub destroy_material: Handle<OutlineMaterial>,

    pub current_hovered_entity: Option<Entity>,
    pub should_reset_on_next_update: bool
}
impl MoveAndDeleteManager {
    pub fn new(asset_server: &AssetServer) -> Self {
        Self {
            move_material: asset_server.add(OutlineMaterial {
                label: "Terrain Placement Move".to_string(),
                color: Color::WHITE,
                ..Default::default()
            }),
            destroy_material: asset_server.add(OutlineMaterial {
                label: "Terrain Placement Destroy".to_string(),
                color: Color::RED,
                ..Default::default()
            }),
            current_hovered_entity: None,
            should_reset_on_next_update: false
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn handle_over_material(
    mut commands: Commands,
    manager: Res<TerrainPlacementManager>,
    mut move_delete_manager: ResMut<MoveAndDeleteManager>,
    (phworld, window, camera_query): (
        Res<PhysicsWorld>,
        Single<&Window, With<PrimaryWindow>>,
        Query<(&GlobalTransform, &CameraView), With<Camera>>
    ),
    collider_element_query: Query<&ChildOf, With<Collider>>,
    buildings_query: Query<(Entity, &ChildOf, &PbrMaterial3d<PbrMaterial>)>,
    children_query: Query<&Children>,
    grid_entities: Query<&GridEntity>
) {
    if move_delete_manager.should_reset_on_next_update {
        reset_hovered_entity(&mut commands, &mut move_delete_manager, children_query);
        move_delete_manager.should_reset_on_next_update = false;
        return;
    }
    if manager.mode != TerrainPlacementMode::Move && manager.mode != TerrainPlacementMode::Remove {
        return;
    }

    // Get cursor position and camera data
    let (Some(cursor_pos), Ok((camera_transform, camera_view))) =
        (window.cursor_position(), camera_query.single())
    else {
        reset_hovered_entity(&mut commands, &mut move_delete_manager, children_query);
        return;
    };

    // Create ray from camera
    let cursor_ndc = cursor_pos / window.size();
    let aspect_ratio = window.size().x / window.size().y;
    let ray = Ray::from_ndc(cursor_ndc, aspect_ratio, camera_transform, camera_view);

    // Cast the ray: modules first, as they are inside other buildings
    let hit = phworld
        .cast_ray(
            &ray,
            &RayCastConfig::with_filter(TERRAIN_MODULES_COLLIDER_GROUP)
        )
        .or_else(|| {
            phworld.cast_ray(
                &ray,
                &RayCastConfig::with_filter(TERRAIN_BUILDINGS_COLLIDER_GROUP)
            )
        });
    let entity = match hit {
        Some((entity, _toi)) => entity,
        None => {
            reset_hovered_entity(&mut commands, &mut move_delete_manager, children_query);
            return;
        }
    };

    // Get the parent building entity and set every other child material to the move/destroy material
    let parent = match collider_element_query.get(entity) {
        Ok(parent) => parent.0,
        Err(_) => return
    };
    if move_delete_manager.current_hovered_entity == Some(parent) {
        return;
    }

    // Clear the outline of the previously hovered building
    reset_hovered_entity(&mut commands, &mut move_delete_manager, children_query);

    // Network pieces are only drawn with the line tool, not moved one by one
    if manager.mode == TerrainPlacementMode::Move
        && grid_entities
            .get(parent)
            .is_ok_and(|g| g.entry().network.is_some())
    {
        return;
    }

    // Outline every mesh of the building (its own material stays, so it remains visible)
    let outline = match manager.mode {
        TerrainPlacementMode::Move => move_delete_manager.move_material.clone(),
        _ => move_delete_manager.destroy_material.clone()
    };
    if let Ok(children) = children_query.get(parent) {
        for child in children.iter() {
            if buildings_query.contains(child) {
                commands
                    .entity(child)
                    .try_insert(PbrMaterial3d(outline.clone()));
            }
        }
    }
    move_delete_manager.current_hovered_entity = Some(parent);
}

#[allow(clippy::too_many_arguments)]
pub fn handle_move_and_delete_update(
    mut commands: Commands,
    mut manager: ResMut<TerrainPlacementManager>,
    mut move_delete_manager: ResMut<MoveAndDeleteManager>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    children_query: Query<&Children>,
    entity_label_query: Query<&Name>,
    config: Res<PlacementConfig>,
    mut grid_entity_events: MessageWriter<GridEntityEvent>
) {
    if move_delete_manager.current_hovered_entity.is_none()
        || (manager.mode != TerrainPlacementMode::Move
            && manager.mode != TerrainPlacementMode::Remove)
    {
        return;
    }
    if mouse_input.just_pressed(MouseButton::Left) {
        let hovered_entity = move_delete_manager.current_hovered_entity.unwrap();

        // Reset hovered entity
        reset_hovered_entity(&mut commands, &mut move_delete_manager, children_query);

        // Handle move
        if manager.mode == TerrainPlacementMode::Move {
            // Get the entry corresponding to the hovered entity
            let entry = config
                .entries
                .iter()
                .find(|e| e.label == entity_label_query.get(hovered_entity).unwrap().as_str());
            let Some(entry) = entry else {
                error!("Could not find placement config entry for hovered entity");
                return;
            };
            manager.new_entry_to_place = Some(entry.clone());
        }

        // Despawn entity
        commands.entity(hovered_entity).despawn_children();
        commands.entity(hovered_entity).despawn();

        // Notify the grid
        grid_entity_events.write(GridEntityEvent::Removed {
            entity: hovered_entity
        });
    }
}

fn reset_hovered_entity(
    commands: &mut Commands,
    move_delete_manager: &mut ResMut<MoveAndDeleteManager>,
    children_query: Query<&Children>
) {
    if let Some(hovered_entity) = move_delete_manager.current_hovered_entity
        && let Ok(children) = children_query.get(hovered_entity)
    {
        for child in children.iter() {
            commands
                .entity(child)
                .try_remove::<PbrMaterial3d<OutlineMaterial>>();
        }
    }
    move_delete_manager.current_hovered_entity = None;
}
