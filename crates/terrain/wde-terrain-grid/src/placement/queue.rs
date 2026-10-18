use std::collections::HashSet;

use bevy::prelude::*;
use wde_gltf::prelude::*;
use wde_logger::prelude::*;
use wde_terrain::prelude::Terrain;

use crate::{
    core::{
        entries::PlacementConfig,
        grid::{GridEntityEvent, GridTilePos}
    },
    placement::{
        cutaway::BuildingVariant,
        modules::{InteriorModule, module_host, spawn_module},
        place::{on_flat_ground, spawn_building}
    },
    prelude::{Grid, GridEntity, GridRotation}
};

/// Time a queued module waits for a building to host it, in seconds.
const MODULE_HOST_TIMEOUT: f32 = 2.0;

/// Buildings placed from code (e.g. a starting base), as if placed with the placement tool.
/// Buildings are placed together once all their models are loaded; modules then go in the
/// building around them. A building whose tiles are taken, or a module without a building to
/// host it, is skipped with a warning.
#[derive(Resource, Default)]
pub struct PlacementQueue {
    pending: Vec<(String, Vec2, GridRotation)>,
    /// Modules waiting for their host, with the time waited.
    modules: Vec<(GridEntity, f32)>
}
impl PlacementQueue {
    /// Places the placement config entry `label` centered at the world XZ position `center`.
    pub fn place(&mut self, label: impl Into<String>, center: Vec2, rotation: GridRotation) {
        self.pending.push((label.into(), center, rotation));
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn process_placement_queue(
    mut commands: Commands,
    mut queue: ResMut<PlacementQueue>,
    config: Res<PlacementConfig>,
    grid: Res<Grid>,
    gltf_models: Res<Assets<GltfAsset>>,
    buildings: Query<&GridEntity, With<BuildingVariant>>,
    modules: Query<&GridEntity, With<InteriorModule>>,
    terrain: Query<&Terrain>,
    time: Res<Time>,
    mut grid_entity_events: MessageWriter<GridEntityEvent>
) {
    if grid.get_parent().is_none() {
        return;
    }
    place_buildings(
        &mut commands,
        &mut queue,
        &config,
        &grid,
        &gltf_models,
        terrain.single().ok(),
        &mut grid_entity_events
    );

    // Modules, once the buildings around them are on the grid
    let mut placed: Vec<GridEntity> = Vec::new();
    let mut waiting = Vec::new();
    for (grid_entity, waited) in std::mem::take(&mut queue.modules) {
        let others = modules.iter().chain(placed.iter());
        match module_host(&grid, &buildings, others, &grid_entity) {
            Some((host, floor)) => {
                spawn_module(
                    &mut commands,
                    &grid,
                    &gltf_models,
                    &grid_entity,
                    host,
                    floor
                );
                placed.push(grid_entity);
            }
            None if waited < MODULE_HOST_TIMEOUT => {
                waiting.push((grid_entity, waited + time.delta_secs()));
            }
            None => warn!(
                "Cannot place module '{}' at {}: no building interior to host it there.",
                grid_entity.entry().label,
                grid_entity.center()
            )
        }
    }
    queue.modules = waiting;
}

/// Places the queued buildings, and moves the queued modules to their waiting list.
fn place_buildings(
    commands: &mut Commands,
    queue: &mut PlacementQueue,
    config: &PlacementConfig,
    grid: &Grid,
    gltf_models: &Assets<GltfAsset>,
    terrain: Option<&Terrain>,
    grid_entity_events: &mut MessageWriter<GridEntityEvent>
) {
    if queue.pending.is_empty() {
        return;
    }

    // Wait for every entry and model, so that the whole batch is placed in the same frame
    let mut placements = Vec::with_capacity(queue.pending.len());
    for (label, center, rotation) in &queue.pending {
        match config.entries.iter().find(|e| e.label == *label) {
            Some(entry) if gltf_models.get(&entry.asset).is_some() => {
                placements.push(GridEntity::new(*center, *rotation, entry.clone()));
            }
            Some(_) => return,
            None if config.is_loading() => return,
            None => warn!("Cannot place unknown building '{}'.", label)
        }
    }
    queue.pending.clear();

    // Tiles are registered in the grid later on: also check against this batch
    let mut taken: HashSet<GridTilePos> = HashSet::new();
    for grid_entity in placements {
        if grid_entity.entry().module {
            queue.modules.push((grid_entity, 0.0));
            continue;
        }
        let free = grid.is_area_free(&grid_entity)
            && grid_entity.footprint().iter().all(|t| !taken.contains(t));
        if !free {
            warn!(
                "Cannot place '{}' at {}: its tiles are taken.",
                grid_entity.entry().label,
                grid_entity.center()
            );
            continue;
        }
        if !on_flat_ground(terrain, &grid_entity) {
            warn!(
                "Cannot place '{}' at {}: the ground is not flat buildable ground.",
                grid_entity.entry().label,
                grid_entity.center()
            );
            continue;
        }
        taken.extend(grid_entity.footprint().iter().copied());
        spawn_building(
            commands,
            grid,
            gltf_models,
            &grid_entity,
            grid_entity_events
        );
    }
}
