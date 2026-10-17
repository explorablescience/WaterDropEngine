use std::collections::HashSet;

use bevy::prelude::*;
use wde_gltf::prelude::*;
use wde_logger::prelude::*;

use crate::{
    core::{
        entries::PlacementConfig,
        grid::{GridEntityEvent, GridTilePos}
    },
    placement::place::spawn_building,
    prelude::{Grid, GridEntity, GridRotation}
};

/// Buildings placed from code (e.g. a starting base), as if placed with the placement tool. They
/// are placed together once all their models are loaded; a building whose tiles are taken is
/// skipped with a warning.
#[derive(Resource, Default)]
pub struct PlacementQueue {
    pending: Vec<(String, Vec2, GridRotation)>
}
impl PlacementQueue {
    /// Places the placement config entry `label` centered at the world XZ position `center`.
    pub fn place(&mut self, label: impl Into<String>, center: Vec2, rotation: GridRotation) {
        self.pending.push((label.into(), center, rotation));
    }
}

pub(crate) fn process_placement_queue(
    mut commands: Commands,
    mut queue: ResMut<PlacementQueue>,
    config: Res<PlacementConfig>,
    grid: Res<Grid>,
    gltf_models: Res<Assets<GltfAsset>>,
    mut grid_entity_events: MessageWriter<GridEntityEvent>
) {
    if queue.pending.is_empty() || grid.get_parent().is_none() {
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
        taken.extend(grid_entity.footprint().iter().copied());
        spawn_building(
            &mut commands,
            &grid,
            &gltf_models,
            &grid_entity,
            &mut grid_entity_events
        );
    }
}
