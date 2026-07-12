use wde_logger::prelude::*;

use bevy::{prelude::*, window::PrimaryWindow};
use wde_camera::prelude::*;
use wde_pbr::prelude::*;
use wde_pbr_outline::prelude::*;
use wde_physics::prelude::*;
use wde_renderer::prelude::Color;
use wde_terrain::prelude::*;

use crate::{
    core::{entries::PlacementConfig, grid::GridEntityEvent}, placement::{TerrainPlacementManager, TerrainPlacementMode}
};

#[derive(Component)]
pub(crate) struct MaterialStorer(Handle<PbrMaterial>);

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
    buildings_hover_query: Query<&MaterialStorer>,
    children_query: Query<&Children>
) {
    if move_delete_manager.should_reset_on_next_update {
        reset_hovered_entity(
            &mut commands,
            &mut move_delete_manager,
            buildings_hover_query,
            children_query
        );
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
        reset_hovered_entity(
            &mut commands,
            &mut move_delete_manager,
            buildings_hover_query,
            children_query
        );
        return;
    };

    // Create ray from camera
    let cursor_ndc = cursor_pos / window.size();
    let aspect_ratio = window.size().x / window.size().y;
    let ray = Ray::from_ndc(cursor_ndc, aspect_ratio, camera_transform, camera_view);

    // Cast the ray
    let entity = match phworld.cast_ray(
        &ray,
        &RayCastConfig::with_filter(TERRAIN_BUILDINGS_COLLIDER_GROUP)
    ) {
        Some((entity, _toi)) => entity,
        None => {
            reset_hovered_entity(
                &mut commands,
                &mut move_delete_manager,
                buildings_hover_query,
                children_query
            );
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

    // For the old hovered entity, if it exists, remove the move/destroy material and put back the old one
    reset_hovered_entity(
        &mut commands,
        &mut move_delete_manager,
        buildings_hover_query,
        children_query
    );

    // Set all children to new material (move or destroy)
    if let Ok(children) = children_query.get(parent) {
        for child in children.iter() {
            // Get its old pbr material
            let old_material = match buildings_query.get(child) {
                Ok((_, _, pbr_material)) => pbr_material.0.clone(),
                Err(_) => continue
            };

            // Set its material to outline
            commands.entity(child).insert((
                if manager.mode == TerrainPlacementMode::Move {
                    PbrMaterial3d(move_delete_manager.move_material.clone())
                } else {
                    PbrMaterial3d(move_delete_manager.destroy_material.clone())
                },
                MaterialStorer(old_material)
            ));
            commands
                .entity(child)
                .remove::<PbrMaterial3d<PbrMaterial>>();
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
    buildings_hover_query: Query<&MaterialStorer>,
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
        reset_hovered_entity(
            &mut commands,
            &mut move_delete_manager,
            buildings_hover_query,
            children_query
        );

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
    buildings_hover_query: Query<&MaterialStorer>,
    children_query: Query<&Children>
) {
    if let Some(hovered_entity) = move_delete_manager.current_hovered_entity
        && let Ok(children) = children_query.get(hovered_entity)
    {
        for child in children.iter() {
            if let Ok(old_material) = buildings_hover_query.get(child) {
                commands
                    .entity(child)
                    .insert(PbrMaterial3d(old_material.0.clone()));
                commands
                    .entity(child)
                    .remove::<PbrMaterial3d<OutlineMaterial>>();
                commands.entity(child).remove::<MaterialStorer>();
            }
        }
    }
    move_delete_manager.current_hovered_entity = None;
}
