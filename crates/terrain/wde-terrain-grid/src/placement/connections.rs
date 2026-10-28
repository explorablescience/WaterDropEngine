//! Connection state of placed buildings.
//!
//! Two buildings are connected through an anchor when one of their anchors of the same kind
//! (`AnchorIn` for tunnels, `AnchorOut` for paths) meet at the same world position. Models use
//! it to show variant parts, e.g. a dome door is only open when a tunnel is plugged into it, and
//! the [`BuildingGraph`] gathers every link to find the groups of linked buildings.

use std::collections::{BTreeSet, HashMap, VecDeque};

use bevy::prelude::*;

use crate::{
    core::{entries::PlacementAnchor, grid::TILE_SIZE},
    placement::cutaway::BuildingVariant,
    prelude::GridEntity
};

/// Max distance between two anchors to be considered connected.
const ANCHOR_SNAP_DISTANCE: f32 = 0.1;

/// Names of the anchors of a placed building that are connected to a neighbor.
#[derive(Component, Default, Clone, PartialEq, Eq, Debug, Reflect)]
#[reflect(Component)]
pub struct BuildingConnections(pub BTreeSet<String>);

/// Network a link goes through.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
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

/// Link from an anchor of a placed building to the anchor of a neighbor.
#[derive(Clone, Debug, Reflect)]
pub struct BuildingLink {
    pub entity: Entity,
    pub kind: LinkKind,
    /// Anchor of this building.
    pub anchor: String,
    /// Anchor of the neighbor.
    pub neighbor_anchor: String,
    /// World XZ position where the two anchors meet.
    pub position: Vec2
}

/// Links between all placed buildings, and the groups of buildings linked together. Rebuilt in
/// [`BuildingGraphSet`] when a building is placed or removed.
#[derive(Resource, Default, Debug, Reflect)]
#[reflect(Resource)]
pub struct BuildingGraph {
    links: HashMap<Entity, Vec<BuildingLink>>,
    groups: Vec<Vec<Entity>>,
    group_of: HashMap<Entity, usize>
}
impl BuildingGraph {
    /// Builds the groups from the links of every placed building, in entity order.
    fn new(links: HashMap<Entity, Vec<BuildingLink>>) -> Self {
        let mut entities: Vec<Entity> = links.keys().copied().collect();
        entities.sort();
        let mut groups = Vec::new();
        let mut group_of = HashMap::new();
        for start in entities {
            if group_of.contains_key(&start) {
                continue;
            }
            let index = groups.len();
            let mut group = Vec::new();
            let mut queue = VecDeque::from([start]);
            group_of.insert(start, index);
            while let Some(entity) = queue.pop_front() {
                group.push(entity);
                for link in &links[&entity] {
                    if group_of.insert(link.entity, index).is_none() {
                        queue.push_back(link.entity);
                    }
                }
            }
            groups.push(group);
        }
        Self {
            links,
            groups,
            group_of
        }
    }

    /// Every placed building.
    pub fn buildings(&self) -> impl Iterator<Item = Entity> + '_ {
        self.links.keys().copied()
    }

    /// Links of a placed building, empty if it is not placed.
    pub fn links(&self, entity: Entity) -> &[BuildingLink] {
        self.links.get(&entity).map_or(&[], Vec::as_slice)
    }

    /// Whether `a` and `b` are directly linked through `kind` anchors.
    pub fn are_linked(&self, a: Entity, b: Entity, kind: LinkKind) -> bool {
        self.links(a)
            .iter()
            .any(|link| link.entity == b && link.kind == kind)
    }

    /// Groups of buildings linked together, directly or through others.
    pub fn groups(&self) -> &[Vec<Entity>] {
        &self.groups
    }

    /// Group of a placed building, itself included.
    pub fn group_of(&self, entity: Entity) -> Option<&[Entity]> {
        self.group_of
            .get(&entity)
            .map(|index| self.groups[*index].as_slice())
    }
}

/// Updates [`BuildingConnections`] and the [`BuildingGraph`].
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BuildingGraphSet;

/// Recomputes the links of every placed building when one is placed or removed.
pub(crate) fn update_connections(
    mut commands: Commands,
    added: Query<(), Added<BuildingVariant>>,
    mut removed: RemovedComponents<BuildingVariant>,
    buildings: Query<(Entity, &GridEntity, Option<&BuildingConnections>), With<BuildingVariant>>,
    mut graph: ResMut<BuildingGraph>
) {
    if added.is_empty() && removed.read().count() == 0 {
        return;
    }
    let mut all: Vec<(Entity, &GridEntity)> = buildings.iter().map(|(e, g, _)| (e, g)).collect();
    all.sort_by_key(|(e, _)| *e);

    let mut links = HashMap::new();
    for (i, (entity, grid_entity)) in all.iter().enumerate() {
        let others = all
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, (_, g))| *g);
        let building_links: Vec<BuildingLink> = anchor_links(grid_entity, others)
            .into_iter()
            .map(|link| {
                // `link.other` indexes the others, i.e. skips this building
                let other = if link.other < i {
                    link.other
                } else {
                    link.other + 1
                };
                BuildingLink {
                    entity: all[other].0,
                    kind: link.kind,
                    anchor: link.anchor,
                    neighbor_anchor: link.other_anchor,
                    position: link.position
                }
            })
            .collect();
        links.insert(*entity, building_links);
    }

    for (entity, _, current) in &buildings {
        let connected: BTreeSet<String> = links[&entity]
            .iter()
            .map(|link| link.anchor.clone())
            .collect();
        if current.map(|c| &c.0) != Some(&connected) {
            commands
                .entity(entity)
                .insert(BuildingConnections(connected));
        }
    }
    *graph = BuildingGraph::new(links);
}
