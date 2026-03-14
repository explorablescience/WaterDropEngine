use bevy::prelude::*;
use std::collections::HashSet;

use crate::{core::grid::{GridPos, CHUNK_GRID_SUBDIVISIONS}, prelude::{Grid, GridLocalDir}};

/// Local rotation of an entity on the grid around its center, in 90 degree increments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GridRotation {
    #[default]
    R0,
    R45,
    R90,
    R135,
    R180,
    R225,
    R270,
    R315
}
impl GridRotation {
    fn is_rotated_45(&self) -> bool {
        matches!(self, GridRotation::R45 | GridRotation::R135 | GridRotation::R225 | GridRotation::R315)
    }
}

/// Describes the area occupied by an entity on the grid.
#[derive(Component, Clone, Debug)]
pub struct GridEntity {
    footprint: Vec<GridPos>
}
impl GridEntity {
    pub fn new(center: Vec2, extent: UVec2, rotation: GridRotation) -> Self {
        if !rotation.is_rotated_45() {
            return GridEntity { footprint: compute_footprint_straight(center, extent, rotation) };
        }
        return GridEntity { footprint: compute_footprint_45(center, extent, rotation) };
    }


    /// Gets the list of grid tiles that are occupied by this entity footprint.
    pub fn footprint(&self) -> &Vec<GridPos> {
        return &self.footprint;
    }
}

// For non-rotated entities, the footprint is a simple rectangle around the center
fn compute_footprint_straight(center: Vec2, extent: UVec2, rotation: GridRotation) -> Vec<GridPos> {
    // Change extent if rotated
    let extent = match rotation {
        GridRotation::R0 | GridRotation::R180 => extent,
        GridRotation::R90 | GridRotation::R270 => UVec2::new(extent.y, extent.x),
        _ => unreachable!() // We don't support non-90 degree rotations yet
    };

    // Get chunk and local position of the center
    let (chunk_center, local_center) = Grid::world_to_pos(center);
    let chunk_center_world_space = Grid::pos_to_world(chunk_center, local_center)
        - Vec2::new(Grid::tile_size() / 2.0 * (extent.x % 2 == 1) as i32 as f32, Grid::tile_size() / 2.0 * (extent.y % 2 == 1) as i32 as f32); // Convert from tile corner to tile center

    // Add an offset to start from the center of the object
    let offset_to_center_object = Vec2::new(extent.x as f32, extent.y as f32) * Grid::tile_size() / 2.0 - Grid::tile_size() / 2.0;

    // Compute the footprint
    let mut footprint = Vec::new();
    let dir_list = [GridLocalDir::North, GridLocalDir::East, GridLocalDir::West, GridLocalDir::South];
    for x in 0..extent.x {
        for z in 0..extent.y {
            let local_pos = chunk_center_world_space - offset_to_center_object + Vec2::new(x as f32, z as f32) * Grid::tile_size();
            let (chunk_pos, local_pos) = Grid::world_to_pos(local_pos);
            for dir in dir_list {
                footprint.push((chunk_pos, (local_pos.0, local_pos.1, dir)));
            }
        }
    }
    
    footprint
}

// For entities rotated 45 degrees, the footprint is a diamond shape around the center
fn compute_footprint_45(center: Vec2, extent: UVec2, rotation: GridRotation) -> Vec<GridPos> {
    let angle = match rotation {
        GridRotation::R45 => std::f32::consts::FRAC_PI_4,
        GridRotation::R135 => 3.0 * std::f32::consts::FRAC_PI_4,
        GridRotation::R225 => 5.0 * std::f32::consts::FRAC_PI_4,
        GridRotation::R315 => 7.0 * std::f32::consts::FRAC_PI_4,
        _ => unreachable!(),
    };

    // Snap the center to the grid to keep a stable footprint while moving the entity.
    let (chunk_center, local_center) = Grid::world_to_pos(center);
    let center = Grid::pos_to_world(chunk_center, local_center);

    let tile_size = Grid::tile_size();
    let half_extent = Vec2::new(extent.x as f32, extent.y as f32) * tile_size * 0.5;
    let (sin_angle, cos_angle) = angle.sin_cos();

    // Axis-aligned bounds of the rotated rectangle, expanded by one tile to cover border subtiles.
    let aabb_half = Vec2::new(
        cos_angle.abs() * half_extent.x + sin_angle.abs() * half_extent.y,
        sin_angle.abs() * half_extent.x + cos_angle.abs() * half_extent.y,
    ) + Vec2::splat(tile_size);

    let min_chunk = Grid::pos_to_chunk(center - aabb_half);
    let max_chunk = Grid::pos_to_chunk(center + aabb_half);

    let mut footprint = Vec::new();
    let mut visited = HashSet::new();
    let dir_list = [GridLocalDir::North, GridLocalDir::East, GridLocalDir::West, GridLocalDir::South];
    let epsilon = tile_size * 0.01;

    for chunk_x in min_chunk.x..=max_chunk.x {
        for chunk_y in min_chunk.y..=max_chunk.y {
            let chunk_pos = IVec2::new(chunk_x, chunk_y);

            for local_x in 0..CHUNK_GRID_SUBDIVISIONS {
                for local_y in 0..CHUNK_GRID_SUBDIVISIONS {
                    for dir in dir_list {
                        let local_pos = (local_x, local_y, dir);
                        let subtile_world = Grid::pos_to_world(chunk_pos, local_pos);

                        // Convert to the rectangle local space by applying inverse rotation.
                        let delta = subtile_world - center;
                        let local = Vec2::new(
                            delta.x * cos_angle + delta.y * sin_angle,
                            -delta.x * sin_angle + delta.y * cos_angle,
                        );

                        if local.x.abs() <= half_extent.x + epsilon && local.y.abs() <= half_extent.y + epsilon {
                            let grid_pos = (chunk_pos, local_pos);
                            if visited.insert(grid_pos) {
                                footprint.push(grid_pos);
                            }
                        }
                    }
                }
            }
        }
    }

    footprint
}
