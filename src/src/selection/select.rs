use wde::prelude::*;

use bevy::{prelude::*, window::PrimaryWindow};

use crate::{colonist::ENTITIES_COLLIDER_GROUP, selection::{EntitySelectionType, SelectionOutlineMaterial, SelectionState, drag::{self, DragRelease, SelectionDrag}}};

pub struct SelectPlugin;
impl Plugin for SelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (drag::update_drag, update_selected, outline_new_meshes).chain());
    }
}

/// Updates the hovered entity from the cursor, and the selection on click or drag. Shift-clicking
/// a colonist adds it to (or removes it from) the selected colonists, shift-dragging adds them.
fn update_selected(
    mut commands: Commands,
    (drag, keys, input_capture, ui_pointer): (
        Res<SelectionDrag>,
        Res<ButtonInput<KeyCode>>,
        Res<PlacementInputCapture>,
        Res<UiPointer>
    ),
    (window, camera, physics): (
        Single<&Window, With<PrimaryWindow>>,
        Query<(&GlobalTransform, &CameraView), With<Camera>>,
        Res<PhysicsWorld>
    ),
    (collider_parent_query, children_query, mesh_query, existing_query): (
        Query<&ChildOf, With<Collider>>,
        Query<&Children>,
        Query<(), With<Mesh3d>>,
        Query<()>
    ),
    selection_materials: Res<SelectionOutlineMaterial>,
    mut selection_state: ResMut<SelectionState>
) {
    selection_state.selected.retain(|selected| existing_query.contains(selected.entity()));

    // Placement tools and UI own the mouse
    let captured = input_capture.is_captured() || ui_pointer.over_ui;
    let hovered = if captured {
        None
    } else {
        hovered_entity(&window, &camera, &physics, &collider_parent_query)
    };

    // Entities whose outline may have changed
    let mut changed = Vec::new();
    if selection_state.hovered != hovered {
        changed.extend(selection_state.hovered);
        changed.extend(hovered);
        selection_state.hovered = hovered;
    }

    if let Some(release) = &drag.release {
        let additive = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        let previous = std::mem::take(&mut selection_state.selected);
        let previous_colonists = || {
            previous
                .iter()
                .copied()
                .filter(|selected| matches!(selected, EntitySelectionType::Entity(_)))
        };
        selection_state.selected = match (release, hovered) {
            (DragRelease::Area(picked), _) => {
                let picked = picked.iter().copied().map(EntitySelectionType::Entity);
                match additive {
                    true => {
                        let mut colonists: Vec<_> = previous_colonists().collect();
                        for colonist in picked {
                            if !colonists.contains(&colonist) {
                                colonists.push(colonist);
                            }
                        }
                        colonists
                    }
                    false => picked.collect()
                }
            }
            // Colonists are toggled in the selection, which drops any selected building
            (DragRelease::Click, Some(colonist @ EntitySelectionType::Entity(_))) if additive => {
                let mut colonists: Vec<_> = previous_colonists().collect();
                match colonists.iter().position(|selected| *selected == colonist) {
                    Some(index) => {
                        colonists.remove(index);
                    }
                    None => colonists.push(colonist)
                }
                colonists
            }
            (DragRelease::Click, Some(target)) => vec![target],
            (DragRelease::Click, None) if additive => previous.clone(),
            (DragRelease::Click, None) => Vec::new()
        };
        changed.extend(previous);
        changed.extend(selection_state.selected.iter().copied());
    }

    for target in changed {
        let material = selection_materials.get(&selection_state, target.entity());
        set_entity_outline(&mut commands, &children_query, &mesh_query, target.entity(), material);
    }
}

/// Entity under the cursor. Colonists first, then modules as they are inside other buildings.
fn hovered_entity(
    window: &Window,
    camera: &Query<(&GlobalTransform, &CameraView), With<Camera>>,
    physics: &PhysicsWorld,
    collider_parent_query: &Query<&ChildOf, With<Collider>>
) -> Option<EntitySelectionType> {
    let cursor = window.cursor_position()?;
    let (camera_transform, camera_view) = camera.single().ok()?;
    let ray = Ray::from_ndc(
        cursor / window.size(),
        window.size().x / window.size().y,
        camera_transform,
        camera_view
    );

    let (collider, _) = physics
        .cast_ray(&ray, &RayCastConfig::with_filter(ENTITIES_COLLIDER_GROUP))
        .or_else(|| physics.cast_ray(&ray, &RayCastConfig::with_filter(crate::MODULES_COLLIDER_GROUP)))
        .or_else(|| physics.cast_ray(&ray, &RayCastConfig::with_filter(ColliderGroup::all())))?;
    let group = physics.collider_group(collider)?;
    // Colliders are spawned as children of the entity owning the meshes
    let entity = collider_parent_query.get(collider).map(ChildOf::parent).ok()?;
    match group {
        crate::BUILDINGS_COLLIDER_GROUP => Some(EntitySelectionType::Building(entity)),
        crate::MODULES_COLLIDER_GROUP => Some(EntitySelectionType::Module(entity)),
        ENTITIES_COLLIDER_GROUP => Some(EntitySelectionType::Entity(entity)),
        _ => None
    }
}

/// Outlines meshes spawned under a hovered or selected entity (building variant or colonist model swaps).
fn outline_new_meshes(
    mut commands: Commands,
    selection_state: Res<SelectionState>,
    selection_materials: Res<SelectionOutlineMaterial>,
    new_meshes: Query<Entity, Added<Mesh3d>>,
    parents: Query<&ChildOf>
) {
    if selection_state.hovered.is_none() && selection_state.selected.is_empty() {
        return;
    }
    for mesh in &new_meshes {
        if let Some(material) = parents
            .iter_ancestors(mesh)
            .find_map(|ancestor| selection_materials.get(&selection_state, ancestor))
        {
            commands.entity(mesh).try_insert(PbrMaterial3d(material.clone()));
        }
    }
}

/// Sets the outline material for the given entity and its children that have a Mesh3d component
fn set_entity_outline(
    commands: &mut Commands,
    children_query: &Query<&Children>,
    mesh_query: &Query<(), With<Mesh3d>>,
    entity: Entity,
    outline_material: Option<&Handle<OutlineMaterial>>
) {
    for child in children_query.iter_descendants(entity) {
        if !mesh_query.contains(child) {
            continue;
        }

        let mut entity = commands.entity(child);
        match outline_material {
            Some(material) => {
                entity.try_insert(PbrMaterial3d(material.clone()));
            }
            None => {
                entity.try_remove::<PbrMaterial3d<OutlineMaterial>>();
            }
        }
    }
}
