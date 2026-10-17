//! Colony network: which placed buildings are linked to which, and the bases they form.
//!
//! Two buildings are linked when anchors of the same kind meet (tunnels through `AnchorIn`,
//! paths through `AnchorOut`). A group of linked buildings with exactly one control room is a
//! base; a group with several control rooms is a conflicting base. Buildings in a group without
//! control room, or in a conflicting base, are not working.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::prelude::*;
use wde_terrain_grid::{core::grid::TILE_SIZE, prelude::*};

/// Placement role of the building that defines a base.
pub const CONTROL_ROOM_ROLE: &str = "control_room";

/// Link from an anchor of a building to the anchor of a neighbor.
#[derive(Clone, Debug)]
pub struct NavNeighbor {
    pub entity: Entity,
    pub kind: LinkKind,
    /// Anchor of this building.
    pub anchor: String,
    /// Anchor of the neighbor.
    pub neighbor_anchor: String,
    /// World XZ position of the link.
    pub position: Vec2
}

/// A placed building in the colony network.
#[derive(Clone, Debug)]
pub struct NavBuilding {
    pub label: String,
    /// World XZ center.
    pub center: Vec2,
    /// World XZ rectangle covered by the footprint (min, max).
    pub rect: (Vec2, Vec2),
    pub is_control_room: bool,
    /// Tunnel or path piece.
    pub is_network: bool,
    /// Number of tunnel (`AnchorIn`) anchors the building can be linked through.
    pub tunnel_anchors: usize,
    /// Number of path (`AnchorOut`) anchors the building can be linked through.
    pub path_anchors: usize,
    pub neighbors: Vec<NavNeighbor>,
    /// Index in [`NavGrid::bases`], if linked to a control room.
    pub base: Option<usize>
}

/// Linked buildings sharing at least one control room.
#[derive(Clone, Debug, Default)]
pub struct NavBase {
    /// Exactly one for a working base.
    pub control_rooms: Vec<Entity>,
    /// Every building of the base, control rooms included.
    pub buildings: Vec<Entity>
}
impl NavBase {
    /// Whether the base has a single control room, i.e. its buildings work.
    pub fn is_valid(&self) -> bool {
        self.control_rooms.len() == 1
    }
}

/// Colony network of all placed buildings, rebuilt when a building is placed or removed.
#[derive(Resource, Default, Debug)]
pub struct NavGrid {
    pub buildings: HashMap<Entity, NavBuilding>,
    pub bases: Vec<NavBase>,
    /// Groups of linked buildings without any control room: they do not work.
    pub unconnected: Vec<Vec<Entity>>
}
impl NavGrid {
    /// Whether the building belongs to a base with a single control room.
    pub fn is_working(&self, entity: Entity) -> bool {
        self.buildings
            .get(&entity)
            .and_then(|b| b.base)
            .is_some_and(|base| self.bases[base].is_valid())
    }

    /// Every building not linked to any control room.
    pub fn unconnected_buildings(&self) -> impl Iterator<Item = Entity> + '_ {
        self.unconnected.iter().flatten().copied()
    }

    /// Human readable description of every problem preventing buildings from working.
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        for (index, base) in self.bases.iter().enumerate() {
            if base.is_valid() {
                continue;
            }
            let rooms: Vec<String> = base
                .control_rooms
                .iter()
                .map(|e| fmt_pos(self.buildings[e].center))
                .collect();
            problems.push(format!(
                "Base #{} has {} control rooms (at {}). Only one is allowed per base, so its {} \
                 buildings do not work: disconnect or remove the extra control rooms.",
                index + 1,
                base.control_rooms.len(),
                rooms.join(", "),
                base.buildings.len()
            ));
        }
        for group in &self.unconnected {
            let first = &self.buildings[&group[0]];
            if group.len() == 1 && first.tunnel_anchors + first.path_anchors == 0 {
                problems.push(format!(
                    "{} at {} has no connection point, so it can never join a base.",
                    first.label,
                    fmt_pos(first.center)
                ));
                continue;
            }
            let buildings: Vec<&NavBuilding> = group.iter().map(|e| &self.buildings[e]).collect();
            let center = buildings.iter().map(|b| b.center).sum::<Vec2>() / buildings.len() as f32;
            let them = if group.len() == 1 { "it" } else { "them" };
            let hint = match buildings.iter().any(|b| b.tunnel_anchors > 0) {
                true => format!("Link {them} with tunnels to a control room."),
                false => format!("Link {them} with paths to an airlock leading to a control room.")
            };
            problems.push(format!(
                "{} not linked to any control room around {}, not working: {}. {}",
                match group.len() {
                    1 => "1 building is".to_string(),
                    n => format!("{n} buildings are")
                },
                fmt_pos(center),
                summarize(&buildings),
                hint
            ));

            // Common mistake: a path or tunnel passing along a building without ending on it
            for entity in group {
                let building = &self.buildings[entity];
                if building.is_network {
                    continue;
                }
                let alongside = self.buildings.iter().find(|(other, b)| {
                    b.is_network
                        && touches(building.rect, b.rect)
                        && !building.neighbors.iter().any(|n| n.entity == **other)
                });
                if let Some((_, piece)) = alongside {
                    problems.push(format!(
                        "A {} runs alongside {} at {} without ending on it: lines only connect \
                         where they end against a building side.",
                        piece.label.to_lowercase(),
                        building.label,
                        fmt_pos(building.center)
                    ));
                }
            }
        }
        problems
    }

    fn build(placed: &[(Entity, &GridEntity)]) -> Self {
        let mut buildings = HashMap::new();
        for (i, (entity, grid_entity)) in placed.iter().enumerate() {
            let entry = grid_entity.entry();
            let others = placed
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, (_, g))| *g);
            let neighbors = anchor_links(grid_entity, others)
                .into_iter()
                .map(|link| {
                    // `link.other` indexes the others, i.e. skips this building
                    let other = if link.other < i {
                        link.other
                    } else {
                        link.other + 1
                    };
                    NavNeighbor {
                        entity: placed[other].0,
                        kind: link.kind,
                        anchor: link.anchor,
                        neighbor_anchor: link.other_anchor,
                        position: link.position
                    }
                })
                .collect();
            buildings.insert(
                *entity,
                NavBuilding {
                    label: entry.label.clone(),
                    center: grid_entity.center(),
                    rect: footprint_rect(grid_entity),
                    is_control_room: entry.role.as_deref() == Some(CONTROL_ROOM_ROLE),
                    is_network: entry.network.is_some(),
                    tunnel_anchors: entry.anchors_in.len(),
                    path_anchors: entry.anchors_out.len(),
                    neighbors,
                    base: None
                }
            );
        }

        // Group linked buildings, in placement order for stable base numbers
        let mut grid = NavGrid {
            buildings,
            ..Default::default()
        };
        let mut visited = HashSet::new();
        for (start, _) in placed {
            if !visited.insert(*start) {
                continue;
            }
            let mut group = Vec::new();
            let mut queue = VecDeque::from([*start]);
            while let Some(entity) = queue.pop_front() {
                group.push(entity);
                for neighbor in &grid.buildings[&entity].neighbors {
                    if visited.insert(neighbor.entity) {
                        queue.push_back(neighbor.entity);
                    }
                }
            }
            let control_rooms: Vec<Entity> = group
                .iter()
                .copied()
                .filter(|e| grid.buildings[e].is_control_room)
                .collect();
            if control_rooms.is_empty() {
                grid.unconnected.push(group);
                continue;
            }
            let base = grid.bases.len();
            for entity in &group {
                grid.buildings.get_mut(entity).unwrap().base = Some(base);
            }
            grid.bases.push(NavBase {
                control_rooms,
                buildings: group
            });
        }
        grid
    }
}

/// World XZ rectangle covered by a placed building.
fn footprint_rect(grid_entity: &GridEntity) -> (Vec2, Vec2) {
    // `bbox` holds the centers of two opposite footprint tiles
    let (a, b) = grid_entity.bbox();
    let half = Vec2::splat(TILE_SIZE / 2.0);
    (a.min(b) - half, a.max(b) + half)
}

/// Whether two rectangles share part of an edge (touching, not overlapping).
fn touches(a: (Vec2, Vec2), b: (Vec2, Vec2)) -> bool {
    let overlap = a.1.min(b.1) - a.0.max(b.0);
    let eps = 0.05;
    (overlap.x.abs() < eps && overlap.y > eps) || (overlap.y.abs() < eps && overlap.x > eps)
}

/// Formats a world XZ position.
pub fn fmt_pos(pos: Vec2) -> String {
    format!("({:.0}, {:.0})", pos.x, pos.y)
}

/// "2 × Tunnel Straight, Dome Small" style summary of building labels.
fn summarize(buildings: &[&NavBuilding]) -> String {
    let mut counts: Vec<(&str, usize)> = Vec::new();
    for b in buildings {
        match counts.iter_mut().find(|(label, _)| *label == b.label) {
            Some((_, n)) => *n += 1,
            None => counts.push((&b.label, 1))
        }
    }
    counts
        .iter()
        .map(|(label, n)| match n {
            1 => label.to_string(),
            n => format!("{n} × {label}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Rebuilds the [`NavGrid`] when the connections of a building change (placed, removed or a
/// neighbor changed).
pub(crate) fn rebuild_nav_grid(
    changed: Query<(), Changed<BuildingConnections>>,
    mut removed: RemovedComponents<BuildingConnections>,
    placed: Query<(Entity, &GridEntity), With<BuildingConnections>>,
    mut nav_grid: ResMut<NavGrid>
) {
    if changed.is_empty() && removed.read().count() == 0 {
        return;
    }
    let mut placed: Vec<(Entity, &GridEntity)> = placed.iter().collect();
    placed.sort_by_key(|(e, _)| *e);
    *nav_grid = NavGrid::build(&placed);
}
