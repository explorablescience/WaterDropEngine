use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use wde_gizmos::prelude::*;
use wde_renderer::prelude::Color;
use wde_terrain_grid::prelude::*;

pub struct NavMapPlugin;
impl Plugin for NavMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavMap>()
            .add_systems(Update, (handle_grid_events, debug_show_nav_map));
    }
}

#[derive(Resource, Default)]
struct NavMap {
    nodes: Vec<NavMapNode>,
    entity_nodes: HashMap<Entity, u32>,
}

struct NavMapNode {
    position: Vec2,
    connections: Vec<u32>,
    /// Nodes are never removed from `nodes` so a removed entity's node is disconnected and marked dead
    alive: bool,
}

fn handle_grid_events(
    mut grid_events: MessageReader<GridEntityEvent>,
    mut nav_map: ResMut<NavMap>,
) {
    for event in grid_events.read() {
        match event {
            GridEntityEvent::Placed {
                entity,
                grid_entity,
            } => {
                let entry = grid_entity.entry();

                // Add new center
                let new_node_parent_index = nav_map.nodes.len() as u32;
                nav_map.nodes.push(NavMapNode {
                    position: grid_entity.center(),
                    connections: Vec::new(),
                    alive: true,
                });
                nav_map.entity_nodes.insert(*entity, new_node_parent_index);
                for anchor_in in &entry.anchors_in {
                    let position =
                        grid_entity.center() + rotate_anchor(*anchor_in, grid_entity.rotation());

                    // Check if anchor_in is close to any existing node, otherwise create a new one
                    let close_node = get_close_node(&nav_map, position, 0.1).unwrap_or_else(|| {
                        let new_node_index = nav_map.nodes.len() as u32;
                        nav_map.nodes.push(NavMapNode {
                            position,
                            connections: Vec::new(),
                            alive: true,
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
        anchor.y * cos - anchor.x * sin,
    )
}

fn get_close_node(nav_map: &NavMap, position: Vec2, max_distance: f32) -> Option<u32> {
    let mut visited_nodes = HashSet::new();
    let first_alive_node = nav_map
        .nodes
        .iter()
        .enumerate()
        .find(|(_, node)| node.alive)
        .map(|(index, _)| index as u32)?;
    get_close_node_node(
        nav_map,
        position,
        max_distance,
        &mut visited_nodes,
        first_alive_node,
    )
}

fn get_close_node_node(
    nav_map: &NavMap,
    position: Vec2,
    max_distance: f32,
    visited_nodes: &mut HashSet<u32>,
    node_id: u32,
) -> Option<u32> {
    // Get node
    visited_nodes.insert(node_id);

    // Check distance (dead nodes have been removed from the network and can't be matched)
    let node = &nav_map.nodes[node_id as usize];
    if node.alive {
        let distance = node.position.distance(position);
        if distance <= max_distance {
            return Some(node_id);
        }
    }

    // Recursively check children
    for &child_id in &node.connections {
        if !visited_nodes.contains(&child_id)
            && let Some(close_node_id) =
                get_close_node_node(nav_map, position, max_distance, visited_nodes, child_id)
        {
            return Some(close_node_id);
        }
    }
    None
}

/// Draws every node and every connection exactly once.
fn debug_show_nav_map(nav_map: Res<NavMap>, mut gizmos: ResMut<Gizmos>) {
    for (index, node) in nav_map.nodes.iter().enumerate() {
        if !node.alive {
            continue;
        }
        gizmos.cube(
            Transform::from_xyz(node.position.x, 0.2, node.position.y).with_scale(Vec3::splat(0.1)),
            if node.connections.len() <= 1 {
                Color::GREEN
            } else {
                Color::RED
            },
        );

        for &other_id in &node.connections {
            if (other_id as usize) <= index {
                continue;
            }
            let other_node = &nav_map.nodes[other_id as usize];
            if !other_node.alive {
                continue;
            }
            let color = if node.connections.len() <= 1 || other_node.connections.len() <= 1 {
                Color::GREEN
            } else {
                Color::RED
            };
            gizmos.line(
                Vec3::new(node.position.x, 0.2, node.position.y),
                Vec3::new(other_node.position.x, 0.2, other_node.position.y),
                color,
            );
        }
    }
}
