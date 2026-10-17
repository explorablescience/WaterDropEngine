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

/// Network a link goes through.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LinkKind {
    /// Pressurized link, between `AnchorIn` anchors.
    Tunnel,
    /// Outdoor link, between `AnchorOut` anchors.
    Path
}

/// An anchor of a building meeting an anchor of a neighbor.
#[derive(Clone, Debug)]
pub struct AnchorLink {
    pub anchor: String,
    pub kind: LinkKind,
    /// Index of the neighbor in the `others` iterator.
    pub other: usize,
    pub other_anchor: String,
    /// World XZ position where the two anchors meet.
    pub position: Vec2
}

/// Links between the anchors of `target` and same-kind anchors of the `others`.
pub fn anchor_links<'a>(
    target: &GridEntity,
    others: impl Iterator<Item = &'a GridEntity>
) -> Vec<AnchorLink> {
    let entry = target.entry();
    let reach = |g: &GridEntity| g.entry().extent.max_element() as f32 * TILE_SIZE;
    let mut links = Vec::new();
    for (index, other) in others.enumerate() {
        if target.center().distance(other.center()) > reach(target) + reach(other) {
            continue;
        }
        let pairs: [(LinkKind, &Vec<PlacementAnchor>, &Vec<PlacementAnchor>); 2] = [
            (
                LinkKind::Tunnel,
                &entry.anchors_in,
                &other.entry().anchors_in
            ),
            (
                LinkKind::Path,
                &entry.anchors_out,
                &other.entry().anchors_out
            )
        ];
        for (kind, anchors, other_anchors) in pairs {
            for anchor in anchors {
                let pos = target.anchor_world(anchor.pos);
                if let Some(other_anchor) = other_anchors
                    .iter()
                    .find(|o| other.anchor_world(o.pos).distance(pos) <= ANCHOR_SNAP_DISTANCE)
                {
                    links.push(AnchorLink {
                        anchor: anchor.name.clone(),
                        kind,
                        other: index,
                        other_anchor: other_anchor.name.clone(),
                        position: pos
                    });
                }
            }
        }
    }
    links
}

/// Anchors of `target` that meet a same-kind anchor of one of the `others`.
pub fn connected_anchors<'a>(
    target: &GridEntity,
    others: impl Iterator<Item = &'a GridEntity>
) -> BTreeSet<String> {
    anchor_links(target, others)
        .into_iter()
        .map(|link| link.anchor)
        .collect()
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
