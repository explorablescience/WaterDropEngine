//! Modules: buildings placed inside the interior of others, e.g. a garden in a dome.

use std::collections::HashSet;

use bevy::prelude::*;
use wde_gltf::prelude::*;
use wde_pbr::prelude::*;
use wde_physics::prelude::*;
use wde_renderer::prelude::*;
use wde_terrain::TERRAIN_MODULES_COLLIDER_GROUP;

use crate::{
    core::grid::{GridTilePos, TILE_SIZE},
    placement::cutaway::BuildingVariant,
    prelude::{Grid, GridEntity}
};

/// Module placed inside the interior of the `host` building.
#[derive(Component, Clone, Copy, Debug)]
pub struct InteriorModule {
    pub host: Entity
}

/// Host building of a module at `grid_entity`, and the height of its floor. Every tile of the
/// footprint must lie fully inside the interior of the same building, and be free of `modules`.
pub(crate) fn module_host<'a>(
    grid: &Grid,
    buildings: &Query<&GridEntity, With<BuildingVariant>>,
    modules: impl Iterator<Item = &'a GridEntity>,
    grid_entity: &GridEntity
) -> Option<(Entity, f32)> {
    let mut host = None;
    for (chunk, local) in grid_entity.footprint() {
        let entity = grid.get_entity(*chunk, *local)?;
        if host.is_some_and(|h| h != entity) {
            return None;
        }
        host = Some(entity);
    }
    let host = host?;
    let building = buildings.get(host).ok()?;
    let interior = building.entry().interior?;

    let center = building.anchor_world(interior.center);
    let inside = grid_entity.footprint().iter().all(|tile| {
        let tile_center = Grid::get_tile_world_pos(*tile);
        [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .into_iter()
            .all(|(x, y)| {
                (tile_center + Vec2::new(x, y) * TILE_SIZE / 2.0).distance(center)
                    <= interior.radius
            })
    });
    let tiles: HashSet<&GridTilePos> = grid_entity.footprint().iter().collect();
    let free = modules
        .into_iter()
        .all(|m| m.footprint().iter().all(|t| !tiles.contains(t)));
    (inside && free).then_some((host, interior.floor))
}

/// Spawns a module on the floor of its host, with its meshes, collider and lights. Returns
/// `None` if its model is not loaded yet.
pub(crate) fn spawn_module(
    commands: &mut Commands,
    grid: &Grid,
    gltf_models: &Assets<GltfAsset>,
    grid_entity: &GridEntity,
    host: Entity,
    floor: f32
) -> Option<Entity> {
    let entry = grid_entity.entry();
    let model = gltf_models.get(&entry.asset)?;
    let center = grid_entity.center();
    let module = commands
        .spawn((
            Name::new(entry.label.clone()),
            Transform::from_rotation(Quat::from_rotation_y(grid_entity.rotation().rotation()))
                .with_translation(Vec3::new(center.x, floor, center.y)),
            grid_entity.clone(),
            InteriorModule { host },
            ChildOf(grid.get_parent()?)
        ))
        .id();
    for (mesh, material) in model.models_where(|_| false) {
        commands.spawn((
            Name::new(format!("{} Mesh", entry.label)),
            Transform::default(),
            Mesh3d(mesh.clone()),
            PbrMaterial3d(material.clone()),
            CastShadow,
            ChildOf(module)
        ));
    }
    let extent = model.bbox.max - model.bbox.min;
    commands.spawn((
        Name::new("Collider"),
        Transform::from_translation(model.bbox.min + extent / 2.0),
        Collider::from(BoxCollider::new(extent).with_group(TERRAIN_MODULES_COLLIDER_GROUP)),
        ChildOf(module)
    ));
    model.spawn_lights(commands, module);
    Some(module)
}

/// Removes the modules whose host building was removed.
pub(crate) fn remove_orphan_modules(
    mut commands: Commands,
    modules: Query<(Entity, &InteriorModule)>,
    hosts: Query<(), With<BuildingVariant>>
) {
    for (entity, module) in &modules {
        if !hosts.contains(module.host) {
            commands.entity(entity).despawn();
        }
    }
}
