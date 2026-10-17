//! Connection state of placed buildings.
//!
//! Two buildings are connected through an anchor when one of their anchors of the same kind
//! (`AnchorIn` for tunnels, `AnchorOut` for paths) meet at the same world position. Models use
//! it to show variant parts, e.g. a dome door is only open when a tunnel is plugged into it.

use std::collections::BTreeSet;

use bevy::prelude::*;

use crate::{
    core::{entries::PlacementAnchor, grid::TILE_SIZE},
    placement::cutaway::BuildingVariant,
    prelude::GridEntity
};

/// Max distance between two anchors to be considered connected.
const ANCHOR_SNAP_DISTANCE: f32 = 0.1;

/// Names of the anchors of a placed building that are connected to a neighbor.
#[derive(Component, Default, Clone, PartialEq, Eq, Debug)]
pub struct BuildingConnections(pub BTreeSet<String>);

/// Anchors of `target` that meet a same-kind anchor of one of the `others`.
pub fn connected_anchors<'a>(
    target: &GridEntity,
    others: impl Iterator<Item = &'a GridEntity>
) -> BTreeSet<String> {
    let entry = target.entry();
    let reach = |g: &GridEntity| g.entry().extent.max_element() as f32 * TILE_SIZE;
    let mut connected = BTreeSet::new();
    for other in others {
        if target.center().distance(other.center()) > reach(target) + reach(other) {
            continue;
        }
        let pairs: [(&Vec<PlacementAnchor>, &Vec<PlacementAnchor>); 2] = [
            (&entry.anchors_in, &other.entry().anchors_in),
            (&entry.anchors_out, &other.entry().anchors_out)
        ];
        for (anchors, other_anchors) in pairs {
            for anchor in anchors {
                let pos = target.anchor_world(anchor.pos);
                if other_anchors
                    .iter()
                    .any(|o| other.anchor_world(o.pos).distance(pos) <= ANCHOR_SNAP_DISTANCE)
                {
                    connected.insert(anchor.name.clone());
                }
            }
        }
    }
    connected
}

/// Recomputes the connections of every placed building when one is placed or removed.
pub(crate) fn update_connections(
    mut commands: Commands,
    added: Query<(), Added<BuildingVariant>>,
    mut removed: RemovedComponents<BuildingVariant>,
    buildings: Query<(Entity, &GridEntity, Option<&BuildingConnections>), With<BuildingVariant>>
) {
    if added.is_empty() && removed.read().count() == 0 {
        return;
    }
    let all: Vec<(Entity, &GridEntity)> = buildings.iter().map(|(e, g, _)| (e, g)).collect();
    for (entity, grid_entity, current) in &buildings {
        let others = all
            .iter()
            .filter(|(other, _)| *other != entity)
            .map(|(_, g)| *g);
        let connected = connected_anchors(grid_entity, others);
        if current.map(|c| &c.0) != Some(&connected) {
            commands
                .entity(entity)
                .insert(BuildingConnections(connected));
        }
    }
}
