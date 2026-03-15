use bevy::prelude::*;
use std::collections::HashSet;
use wde_gizmos::prelude::*;
use wde_renderer::prelude::*;
use wde_terrain::prelude::CHUNK_SIZE;

use crate::{core::grid::{CHUNK_GRID_SUBDIVISIONS, Grid, GridLocalPos}, editor::ui::PlacementUI, prelude::GridLocalDir, render::grid::cache::{GridGizmo, GridGizmoCache}};

const Y_GRID: f32 = -0.01; // Slightly above terrain to prevent z-fighting

pub fn render_grid_bare(
    mut commands: Commands,
    grid: Res<Grid>,
    assets: Res<AssetServer>,
    mut cache: ResMut<GridGizmoCache>,
    placement_ui: Res<PlacementUI>
) {
    // Check if cache is initialized
    if cache.line_material.is_none() || cache.diagonal_material.is_none() {
        return;
    }

    // Check if grid view is enabled in the UI
    if !placement_ui.enabled || !placement_ui.view_grid || placement_ui.placement_show_entity {
        return;
    }

    // Query materials and meshes
    let line_material = cache.line_material.clone().unwrap();
    // let diagonal_material = cache.diagonal_material.clone().unwrap();

    // For each chunk, ensure we have gizmo entities created and updated based on the grid state
    let cell_w = CHUNK_SIZE / CHUNK_GRID_SUBDIVISIONS as f32;
    let cell_d = CHUNK_SIZE / CHUNK_GRID_SUBDIVISIONS as f32;
    for (&chunk_pos, _) in grid.get_chunks() {
        // Bare grid meshes are chunk-wide and spawned once per chunk.
        // Do not use `chunk_entities` here because occupied-cell rendering also writes into it.
        if cache.chunk_grid_meshes.contains_key(&chunk_pos)
            && cache.chunk_diagonal_meshes.contains_key(&chunk_pos)
        {
            continue;
        }
        let chunk_min_x = (chunk_pos.x as f32 * CHUNK_SIZE) - (CHUNK_SIZE * 0.5);
        let chunk_min_z = (chunk_pos.y as f32 * CHUNK_SIZE) - (CHUNK_SIZE * 0.5);
        let mut bare_gizmo_entities = Vec::new();

        // Create grid lines mesh and entity
        let grid_mesh = cache
            .chunk_grid_meshes
            .entry(chunk_pos)
            .or_insert_with(|| {
                assets.add(GridGizmoCache::create_grid_lines_mesh(
                    chunk_pos,
                    chunk_min_x,
                    chunk_min_z,
                    cell_w,
                    cell_d,
                ))
            })
            .clone();
        let grid_entity = commands
            .spawn((
                Transform::from_xyz(0.0, Y_GRID, 0.0),
                Mesh(grid_mesh),
                GizmoMaterial(line_material.clone()),
                GridGizmo
            ))
            .id();
        bare_gizmo_entities.push(grid_entity);

        // Create diagonal lines mesh and entity
        // let diagonal_mesh = cache
        //     .chunk_diagonal_meshes
        //     .entry(chunk_pos)
        //     .or_insert_with(|| {
        //         assets.add(GridGizmoCache::create_diagonal_lines_mesh(
        //             chunk_pos,
        //             chunk_min_x,
        //             chunk_min_z,
        //             cell_w,
        //             cell_d,
        //         ))
        //     })
        //     .clone();
        // let diagonal_entity = commands
        //     .spawn((
        //         Transform::from_xyz(0.0, Y_GRID, 0.0),
        //         Mesh(diagonal_mesh),
        //         GizmoMaterial(diagonal_material.clone()),
        //         GridGizmo
        //     ))
        //     .id();
        // bare_gizmo_entities.push(diagonal_entity);

        // Add the bare grid entities without overwriting occupied-cell entities for the same chunk.
        cache
            .chunk_entities
            .entry(chunk_pos)
            .or_default()
            .extend(bare_gizmo_entities);
    }
}

#[derive(Component)]
pub struct RenderGridOccupiedCellsMarker;

pub fn render_grid_occupied_cells(
    mut commands: Commands,
    grid: Res<Grid>,
    mut cache: ResMut<GridGizmoCache>,
    placement_ui: Res<PlacementUI>,
    marker_query: Query<Entity, With<RenderGridOccupiedCellsMarker>>
) {
    // Check if cache is initialized
    if cache.occupied_cell_material.is_none() {
        return;
    }

    // Check if grid view is enabled in the UI
    if !placement_ui.enabled || !placement_ui.placement_show_entity {
        if !cache.occupied_cell_entities.is_empty() {
            for entity in marker_query.iter() {
                commands.entity(entity).despawn();
            }
            cache.occupied_cell_entities.clear();
        }
        return;
    }

    // Query material and mesh for occupied cell indicators
    let occupied_cell_material = cache.occupied_cell_material.clone().unwrap();
    let occupied_cell_mesh = cache.occupied_cell_mesh.clone().unwrap();
    let cell_w = CHUNK_SIZE / CHUNK_GRID_SUBDIVISIONS as f32;
    let cell_d = CHUNK_SIZE / CHUNK_GRID_SUBDIVISIONS as f32;
    let mut desired_tiles = HashSet::new();

    for (&chunk_pos, chunk_data) in grid.get_chunks() {
        // The tile buffer stores 4 directional entries per grid cell.
        for (tile_idx, tile_entity) in chunk_data.get_tiles().iter().enumerate() {
            if let Some(_entity) = tile_entity {
                let cell_idx = (tile_idx as u32) / 4;
                let local_x = cell_idx % CHUNK_GRID_SUBDIVISIONS;
                let local_z = cell_idx / CHUNK_GRID_SUBDIVISIONS;
                let cell_type = match tile_idx % 4 {
                    0 => GridLocalDir::North,
                    1 => GridLocalDir::East,
                    2 => GridLocalDir::West,
                    3 => GridLocalDir::South,
                    _ => continue,
                };
                let local_pos: GridLocalPos = (local_x, local_z, cell_type);
                let cache_key = (chunk_pos, local_pos);
                desired_tiles.insert(cache_key);

                if cache.occupied_cell_entities.contains_key(&cache_key) {
                    continue;
                }

                let cell_pos = Vec3::new(
                    (chunk_pos.x as f32 * CHUNK_SIZE) + (local_x as f32 * cell_w) + (cell_w * 0.5)
                        - (CHUNK_SIZE * 0.5),
                    Y_GRID,
                    (chunk_pos.y as f32 * CHUNK_SIZE) + (local_z as f32 * cell_d) + (cell_d * 0.5)
                        - (CHUNK_SIZE * 0.5),
                );
                let occupied_entity = commands
                    .spawn((
                        Transform::IDENTITY.with_rotation(match cell_type {
                            GridLocalDir::North => Quat::from_rotation_y(0.0),
                            GridLocalDir::South => Quat::from_rotation_y(std::f32::consts::PI),
                            GridLocalDir::West => Quat::from_rotation_y(-std::f32::consts::PI / 2.0),
                            GridLocalDir::East => Quat::from_rotation_y(std::f32::consts::PI / 2.0),
                        }).with_translation(cell_pos + match cell_type {
                            GridLocalDir::North => Vec3::new(0.0, 0.0, 0.9),
                            GridLocalDir::South => Vec3::new(0.0, 0.0, -0.9),
                            GridLocalDir::West => Vec3::new(-0.9, 0.0, 0.0),
                            GridLocalDir::East => Vec3::new(0.9, 0.0, 0.0),
                        }),
                        Mesh(occupied_cell_mesh.clone()),
                        GizmoMaterial(occupied_cell_material.clone()),
                        GridGizmo,
                        RenderGridOccupiedCellsMarker
                    ))
                    .id();

                cache.occupied_cell_entities.insert(cache_key, occupied_entity);
            }
        }
    }

    let stale_tiles: Vec<_> = cache
        .occupied_cell_entities
        .iter()
        .filter_map(|(tile_key, &entity)| {
            if desired_tiles.contains(tile_key) {
                None
            } else {
                Some((*tile_key, entity))
            }
        })
        .collect();

    for (tile_key, entity) in stale_tiles {
        commands.entity(entity).despawn();
        cache.occupied_cell_entities.remove(&tile_key);
    }
}
