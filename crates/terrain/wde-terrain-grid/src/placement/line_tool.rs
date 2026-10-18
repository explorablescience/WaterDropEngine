//! Drag-to-draw tool for networks (tunnels, paths).
//!
//! The user drags an L-shaped route on the grid. Each tile gets the straight or junction piece of
//! the network matching the directions it links to: the route itself, anchors of neighbors facing
//! it (dome doors, airlocks...) and existing pieces the route crosses or runs into, which are
//! upgraded to junctions. Corners, T-junctions and dead ends then come from the piece variants.

use std::collections::{BTreeSet, HashMap};

use bevy::prelude::*;
use wde_editor::prelude::*;
use wde_gltf::prelude::*;
use wde_terrain::prelude::Terrain;

use crate::{
    core::{
        entries::{PlacementConfig, PlacementConfigEntry},
        grid::{GridEntityEvent, TILE_SIZE}
    },
    placement::{
        TerrainCursorPos, TerrainPlacementManager, TerrainPlacementMode,
        connections::connected_anchors,
        cutaway::BuildingVariant,
        place::{on_flat_ground, remove_building, spawn_building, spawn_ghost_parts}
    },
    prelude::{Grid, GridEntity, GridRotation}
};

/// Tile steps, indexed by direction bit: +X, -X, +Z, -Z.
const DIRS: [Vec2; 4] = [
    Vec2::new(TILE_SIZE, 0.0),
    Vec2::new(-TILE_SIZE, 0.0),
    Vec2::new(0.0, TILE_SIZE),
    Vec2::new(0.0, -TILE_SIZE)
];
const AXIS_X: u8 = 0b0011;
const AXIS_Z: u8 = 0b1100;

/// Piece of a planned line: a new piece, or the replacement of an existing one.
pub(crate) struct PlannedPiece {
    pub replaces: Option<Entity>,
    /// The tile is taken by another building: the whole line cannot be built.
    pub blocked: bool,
    pub grid_entity: GridEntity,
    pub connected: BTreeSet<String>
}

/// Line drawing state, while a network is selected in the build menu.
#[derive(Default)]
pub(crate) struct LineToolState {
    /// Tile where the current drag started.
    start: Option<Vec2>,
    /// Signature of the previewed plan, to only respawn the preview when it changes.
    preview_key: Vec<(bool, IVec2, String, GridRotation, BTreeSet<String>)>
}

fn tile_key(center: Vec2) -> IVec2 {
    (center * 10.0).round().as_ivec2()
}

fn snap(pos: Vec2) -> Vec2 {
    Grid::get_tile_world_pos(Grid::get_nearest_tile(pos))
}

fn dir_bit(step: Vec2) -> u8 {
    // Thresholds rather than sign comparisons: negated steps carry -0.0 components
    if step.x > 0.5 {
        1 << 0
    } else if step.x < -0.5 {
        1 << 1
    } else if step.y > 0.5 {
        1 << 2
    } else {
        1 << 3
    }
}

/// L-shaped route of tile centers from `start` to `end`, along the longest axis first.
fn route(start: Vec2, end: Vec2) -> Vec<Vec2> {
    let delta = ((end - start) / TILE_SIZE).round();
    let (nx, nz) = (delta.x as i32, delta.y as i32);
    let legs = if nx.abs() >= nz.abs() {
        [(Vec2::X, nx), (Vec2::Y, nz)]
    } else {
        [(Vec2::Y, nz), (Vec2::X, nx)]
    };
    let mut tiles = vec![start];
    let mut current = start;
    for (axis, n) in legs {
        for _ in 0..n.abs() {
            current += axis * TILE_SIZE * n.signum() as f32;
            tiles.push(current);
        }
    }
    tiles
}

/// Straight and junction entries of a network.
fn network_entries<'a>(
    config: &'a PlacementConfig,
    network: &str
) -> Option<(&'a PlacementConfigEntry, &'a PlacementConfigEntry)> {
    let piece = |junction: bool| {
        config.entries.iter().find(|e| {
            e.network
                .as_ref()
                .is_some_and(|n| n.name == network && n.junction == junction)
        })
    };
    Some((piece(false)?, piece(true)?))
}

/// Directions a placed network piece already links to.
fn piece_mask(piece: &GridEntity) -> u8 {
    match piece.entry().network.as_ref().is_some_and(|n| n.junction) {
        true => AXIS_X | AXIS_Z,
        false => match piece.rotation() {
            GridRotation::R0 | GridRotation::R180 => AXIS_X,
            GridRotation::R90 | GridRotation::R270 => AXIS_Z
        }
    }
}

/// Piece and rotation linking the given directions (straight pieces run along local X).
fn choose_piece<'a>(
    mask: u8,
    straight: &'a PlacementConfigEntry,
    junction: &'a PlacementConfigEntry
) -> (&'a PlacementConfigEntry, GridRotation) {
    if mask & AXIS_Z == 0 {
        (straight, GridRotation::R0)
    } else if mask & AXIS_X == 0 {
        (straight, GridRotation::R90)
    } else {
        (junction, GridRotation::R0)
    }
}

/// Links tile `a` and its neighbor `a + step` (registering new tiles in route order).
fn link(masks: &mut HashMap<IVec2, u8>, order: &mut Vec<Vec2>, a: Vec2, step: Vec2) {
    for (tile, bit) in [(a, dir_bit(step)), (a + step, dir_bit(-step))] {
        let mask = masks.entry(tile_key(tile)).or_insert_with(|| {
            order.push(tile);
            0
        });
        *mask |= bit;
    }
}

/// Plans the pieces to place to draw `network` from tile `start` to tile `end`.
pub(crate) fn plan_line(
    start: Vec2,
    end: Vec2,
    network: &str,
    config: &PlacementConfig,
    grid: &Grid,
    buildings: &Query<(Entity, &GridEntity), With<BuildingVariant>>
) -> Vec<PlannedPiece> {
    let Some((straight, junction)) = network_entries(config, network) else {
        return Vec::new();
    };
    let uses_in = !straight.anchors_in.is_empty();
    let occupant = |center: Vec2| {
        let (chunk, local) = Grid::get_nearest_tile(center);
        grid.get_entity(chunk, local)
            .and_then(|e| buildings.get(e).ok())
    };
    let in_network = |g: &GridEntity| {
        g.entry()
            .network
            .as_ref()
            .is_some_and(|n| n.name == network)
    };

    // Directions each tile must link to, in route order
    let mut order: Vec<Vec2> = Vec::new();
    let mut masks: HashMap<IVec2, u8> = HashMap::new();
    let tiles = route(start, end);
    let endpoints = [tile_key(tiles[0]), tile_key(*tiles.last().unwrap())];
    for pair in tiles.windows(2) {
        link(&mut masks, &mut order, pair[0], pair[1] - pair[0]);
    }
    if tiles.len() == 1 {
        order.push(start);
        masks.insert(tile_key(start), 0);
    }
    // Running into the side of an existing piece makes a T-junction
    if tiles.len() >= 2 {
        let n = tiles.len();
        for (tile, step) in [
            (tiles[n - 1], tiles[n - 1] - tiles[n - 2]),
            (tiles[0], tiles[0] - tiles[1])
        ] {
            if occupant(tile).is_none()
                && let Some((_, g)) = occupant(tile + step)
                && in_network(g)
            {
                link(&mut masks, &mut order, tile, step);
            }
        }
    }

    // Pick each tile's piece
    let mut pieces = Vec::new();
    for center in order {
        let mask = masks[&tile_key(center)];
        match occupant(center) {
            Some((entity, existing)) if in_network(existing) => {
                let current = piece_mask(existing);
                if mask | current == current {
                    continue; // Already links everywhere needed
                }
                let (entry, rotation) = choose_piece(mask | current, straight, junction);
                pieces.push(PlannedPiece {
                    replaces: Some(entity),
                    blocked: false,
                    grid_entity: GridEntity::new(center, rotation, entry.clone()),
                    connected: BTreeSet::new()
                });
            }
            Some(_) => {
                // Another building is in the way
                let (entry, rotation) = choose_piece(mask, straight, junction);
                pieces.push(PlannedPiece {
                    replaces: None,
                    blocked: true,
                    grid_entity: GridEntity::new(center, rotation, entry.clone()),
                    connected: BTreeSet::new()
                });
            }
            None => {
                // The ends of the line also link to neighbors with an anchor on the shared edge
                // (doors, building accesses, existing pieces). Not the middle: a line running
                // alongside a building would get a junction at every tile.
                let mut mask = mask;
                let is_end = endpoints.contains(&tile_key(center));
                for (i, step) in DIRS.iter().enumerate().filter(|_| is_end) {
                    let edge = center + *step / 2.0;
                    if let Some((_, g)) = occupant(center + *step) {
                        let anchors = match uses_in {
                            true => &g.entry().anchors_in,
                            false => &g.entry().anchors_out
                        };
                        if anchors
                            .iter()
                            .any(|a| g.anchor_world(a.pos).distance(edge) < 0.1)
                        {
                            mask |= 1 << i;
                        }
                    }
                }
                let (entry, rotation) = choose_piece(mask, straight, junction);
                pieces.push(PlannedPiece {
                    replaces: None,
                    blocked: false,
                    grid_entity: GridEntity::new(center, rotation, entry.clone()),
                    connected: BTreeSet::new()
                });
            }
        }
    }

    // Variant each piece will have once everything is placed
    let replaced: Vec<Entity> = pieces.iter().filter_map(|p| p.replaces).collect();
    let mut world: Vec<GridEntity> = buildings
        .iter()
        .filter(|(e, _)| !replaced.contains(e))
        .map(|(_, g)| g.clone())
        .collect();
    let first_planned = world.len();
    world.extend(pieces.iter().map(|p| p.grid_entity.clone()));
    for (i, piece) in pieces.iter_mut().enumerate() {
        let others = world
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != first_planned + i)
            .map(|(_, g)| g);
        piece.connected = connected_anchors(&piece.grid_entity, others);
    }
    pieces
}

/// Drag to draw the selected network: press to start, release to build, right click to cancel.
#[allow(clippy::too_many_arguments)]
pub(crate) fn line_tool_update(
    mut commands: Commands,
    mut manager: ResMut<TerrainPlacementManager>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    cursor: Res<TerrainCursorPos>,
    ui: Option<Res<UIContext>>,
    config: Res<PlacementConfig>,
    grid: Res<Grid>,
    gltf_models: Res<Assets<GltfAsset>>,
    asset_server: Res<AssetServer>,
    buildings: Query<(Entity, &GridEntity), With<BuildingVariant>>,
    terrain: Query<&Terrain>,
    mut grid_entity_events: MessageWriter<GridEntityEvent>
) {
    let network = match (&manager.mode, &manager.line_network) {
        (TerrainPlacementMode::Place, Some(network)) => network.clone(),
        _ => {
            if manager.line_tool.start.is_some() || !manager.line_tool.preview_key.is_empty() {
                clear_preview(&mut commands, &mut manager);
                manager.line_tool.start = None;
            }
            return;
        }
    };
    let over_ui = ui.is_some_and(|ui| ui.0.is_pointer_over_area());
    let Some(cursor_tile) = cursor.pos().map(|p| snap(Vec2::new(p.x, p.z))) else {
        return;
    };

    // Start / cancel the drag
    if mouse_input.just_pressed(MouseButton::Left) && !over_ui {
        manager.line_tool.start = Some(cursor_tile);
    }
    if mouse_input.just_pressed(MouseButton::Right) || keys.just_pressed(KeyCode::Escape) {
        manager.line_tool.start = None;
    }

    // Plan from the drag start, or show a single piece under the cursor
    let start = manager.line_tool.start.unwrap_or(cursor_tile);
    let mut plan = plan_line(start, cursor_tile, &network, &config, &grid, &buildings);
    for piece in plan.iter_mut() {
        piece.blocked |= !on_flat_ground(terrain.single().ok(), &piece.grid_entity);
    }
    let valid = plan.iter().all(|p| !p.blocked);

    if mouse_input.just_released(MouseButton::Left) && manager.line_tool.start.is_some() {
        if !valid {
            // Blocked lines are not built
            manager.line_tool.start = None;
            clear_preview(&mut commands, &mut manager);
            return;
        }
        for piece in &plan {
            if let Some(entity) = piece.replaces {
                remove_building(&mut commands, entity, &mut grid_entity_events);
            }
            spawn_building(
                &mut commands,
                &grid,
                &gltf_models,
                &piece.grid_entity,
                &mut grid_entity_events
            );
        }
        manager.line_tool.start = None;
        clear_preview(&mut commands, &mut manager);
        return;
    }

    // Preview ghosts, rebuilt only when the plan changes
    let key: Vec<_> = plan
        .iter()
        .map(|p| {
            (
                valid,
                tile_key(p.grid_entity.center()),
                p.grid_entity.entry().label.clone(),
                p.grid_entity.rotation(),
                p.connected.clone()
            )
        })
        .collect();
    if key == manager.line_tool.preview_key {
        return;
    }
    clear_preview(&mut commands, &mut manager);
    for piece in &plan {
        let entry = piece.grid_entity.entry();
        let Some(model) = gltf_models.get(&entry.asset) else {
            return; // Retry once loaded
        };
        let center = piece.grid_entity.center();
        let ghost = commands
            .spawn((
                Name::new(format!("Line Preview - {}", entry.label)),
                Transform::from_rotation(Quat::from_rotation_y(
                    piece.grid_entity.rotation().rotation()
                ))
                .with_translation(Vec3::new(center.x, 0.0, center.y)),
                ChildOf(manager.line_preview_root)
            ))
            .id();
        spawn_ghost_parts(
            &mut commands,
            entry,
            model,
            &asset_server,
            ghost,
            &piece.connected,
            valid
        );
    }
    manager.line_tool.preview_key = key;
}

fn clear_preview(commands: &mut Commands, manager: &mut TerrainPlacementManager) {
    commands
        .entity(manager.line_preview_root)
        .despawn_children();
    manager.line_tool.preview_key.clear();
}
