use std::collections::HashMap;

use bevy::prelude::*;
use wde_terrain_grid::prelude::*;

use crate::navgrid::NavMap;

pub struct NavMapInteriorPlugin;
impl Plugin for NavMapInteriorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_grid_events);
    }
}

#[derive(Default)]
pub struct InteriorNavMap {
    pub nodes: Vec<InteriorNavMapNode>,
    pub entity_nodes: HashMap<Entity, u32>
}

pub struct InteriorNavMapNode {
    pub position: Vec2,
    pub connections: Vec<u32>,
    /// Nodes are never removed from `nodes` so a removed entity's node is disconnected and marked dead
    pub alive: bool
}

fn handle_grid_events(
    mut grid_events: MessageReader<GridEntityEvent>,
    mut nav_map: ResMut<NavMap>
) {
    let nav_map = &mut nav_map.interior;
    for event in grid_events.read() {
        match event {
            GridEntityEvent::Placed {
                entity,
                grid_entity
            } => {
                let entry = grid_entity.entry();

                // If only anchors_out are defined, ignore this entity (handled by exterior map)
                if entry.anchors_in.is_empty() {
                    continue;
                }

                // Add new center
                let new_node_parent_index = nav_map.nodes.len() as u32;
                nav_map.nodes.push(InteriorNavMapNode {
                    position: grid_entity.center(),
                    connections: Vec::new(),
                    alive: true
                });
                nav_map.entity_nodes.insert(*entity, new_node_parent_index);
                for anchor_in in &entry.anchors_in {
                    let position =
                        grid_entity.center() + rotate_anchor(*anchor_in, grid_entity.rotation());

                    // Check if anchor_in is close to any existing node, otherwise create a new one
                    let close_node = get_close_node(nav_map, position, 0.1).unwrap_or_else(|| {
                        let new_node_index = nav_map.nodes.len() as u32;
                        nav_map.nodes.push(InteriorNavMapNode {
                            position,
                            connections: Vec::new(),
                            alive: true
                        });
                        new_node_index
                    });

                    // Connect the new center to the anchor node on both ends
                    nav_map.nodes[new_node_parent_index as usize]
                        .connections
                        .push(close_node);
                    nav_map.nodes[close_node as usize]
                        .connections
                        .push(new_node_parent_index);
                }
            }
            GridEntityEvent::Removed { entity } => {
                let Some(node_id) = nav_map.entity_nodes.remove(entity) else {
                    continue;
                };

                // Detach this entity's center from every node it was connected to
                let neighbor_ids = std::mem::take(&mut nav_map.nodes[node_id as usize].connections);
                nav_map.nodes[node_id as usize].alive = false;
                for neighbor_id in neighbor_ids {
                    nav_map.nodes[neighbor_id as usize]
                        .connections
                        .retain(|&id| id != node_id);

                    // An anchor node that only existed to link to this entity is now orphaned
                    let is_entity_center =
                        nav_map.entity_nodes.values().any(|&id| id == neighbor_id);
                    if !is_entity_center
                        && nav_map.nodes[neighbor_id as usize].connections.is_empty()
                    {
                        nav_map.nodes[neighbor_id as usize].alive = false;
                    }
                }
            }
        }
    }
}

/// Rotates a local anchor offset (x maps to world X, y maps to world Z) by the entity's
/// grid rotation, matching the `Quat::from_rotation_y` convention used when placing entities.
fn rotate_anchor(anchor: Vec2, rotation: GridRotation) -> Vec2 {
    let (sin, cos) = rotation.rotation().sin_cos();
    Vec2::new(
        anchor.x * cos + anchor.y * sin,
        anchor.y * cos - anchor.x * sin
    )
}

/// Finds the closest alive node to `position` within `max_distance`
fn get_close_node(nav_map: &InteriorNavMap, position: Vec2, max_distance: f32) -> Option<u32> {
    nav_map
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.alive)
        .map(|(index, node)| (index as u32, node.position.distance(position)))
        .filter(|&(_, distance)| distance <= max_distance)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(index, _)| index)
}
