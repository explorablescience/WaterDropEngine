use bevy::prelude::*;

use crate::core::grid::{GridChunkPos, GridLocalDir, GridLocalPos, CHUNK_GRID_SUBDIVISIONS};

/// Local rotation of an entity on the grid around its center, in 90 degree increments.
#[derive(Clone, Copy, Debug, Default)]
pub enum GridEntityRotation {
    #[default]
    R0,
    R45,
    R90,
    R135,
    R180,
    R225,
    R270,
    R315,
}

/// Describes the area occupied by an entity on the grid.
#[derive(Component, Clone, Debug)]
pub struct GridEntity {
    /// The center position of the entity in world coordinates.
    pub center: Vec2,
    /// The size of the entity's footprint on the grid (width and depth).
    /// Setting a size of (sx, sy) means that the entity occupies the area from (center.x - sx/2, center.y - sy/2) to (center.x + sx/2, center.y + sy/2).
    pub size: Vec2,
    /// Local rotation of the footprint around its center.
    pub rotation: GridEntityRotation,
}
impl GridEntity {
    /// Gets the list of grid tiles that are occupied by this entity footprint.
    pub(crate) fn get_occupied_tiles(&self) -> Vec<(GridChunkPos, GridLocalPos)> {
        let mut tiles = Vec::new();
        let half_size = self.size * 0.5;
        let rotation = self.rotation.as_radians();
        let (sin_r, cos_r) = rotation.sin_cos();
        let axis_x = Vec2::new(cos_r, sin_r);
        let axis_z = Vec2::new(-sin_r, cos_r);

        let rotated_corners = [
            Vec2::new(-half_size.x, -half_size.y),
            Vec2::new(half_size.x, -half_size.y),
            Vec2::new(half_size.x, half_size.y),
            Vec2::new(-half_size.x, half_size.y),
        ].map(|corner| Self::rotate(corner, cos_r, sin_r) + self.center);

        let min_x = rotated_corners.iter().map(|corner| corner.x).fold(f32::INFINITY, f32::min);
        let max_x = rotated_corners.iter().map(|corner| corner.x).fold(f32::NEG_INFINITY, f32::max);
        let min_z = rotated_corners.iter().map(|corner| corner.y).fold(f32::INFINITY, f32::min);
        let max_z = rotated_corners.iter().map(|corner| corner.y).fold(f32::NEG_INFINITY, f32::max);

        // Expand one grid cell around the rotated AABB so boundary subcells are tested.
        let grid_min_x = min_x.floor() as i32 - 1;
        let grid_max_x = max_x.ceil() as i32;
        let grid_min_z = min_z.floor() as i32 - 1;
        let grid_max_z = max_z.ceil() as i32;

        for x in grid_min_x..=grid_max_x {
            for z in grid_min_z..=grid_max_z {
                for (dir, center_offset) in [
                    (GridLocalDir::North, Vec2::new(0.25, 0.25)),
                    (GridLocalDir::East, Vec2::new(0.75, 0.25)),
                    (GridLocalDir::South, Vec2::new(0.25, 0.75)),
                    (GridLocalDir::West, Vec2::new(0.75, 0.75)),
                ] {
                    let subtile_center = Vec2::new(x as f32, z as f32) + center_offset;
                    if Self::rotated_rect_overlaps_subtile(
                        self.center,
                        half_size,
                        axis_x,
                        axis_z,
                        subtile_center,
                    ) {
                        let (chunk_pos, local_x, local_z) = Self::grid_to_chunk_local(x, z);
                        tiles.push((chunk_pos, (local_x, local_z, dir)));
                    }
                }
            }
        }

        tiles
    }

    fn rotate(point: Vec2, cos_r: f32, sin_r: f32) -> Vec2 {
        Vec2::new(
            point.x * cos_r - point.y * sin_r,
            point.x * sin_r + point.y * cos_r,
        )
    }

    fn rotated_rect_overlaps_subtile(
        rect_center: Vec2,
        rect_half_size: Vec2,
        rect_axis_x: Vec2,
        rect_axis_z: Vec2,
        subtile_center: Vec2,
    ) -> bool {
        let subtile_half_size = Vec2::splat(0.25);
        let delta = subtile_center - rect_center;
        let epsilon = 1e-5;

        let translation_x = delta.dot(rect_axis_x).abs();
        let translation_z = delta.dot(rect_axis_z).abs();
        let square_projection_on_rect_x = subtile_half_size.x * rect_axis_x.x.abs()
            + subtile_half_size.y * rect_axis_x.y.abs();
        let square_projection_on_rect_z = subtile_half_size.x * rect_axis_z.x.abs()
            + subtile_half_size.y * rect_axis_z.y.abs();
        if translation_x >= rect_half_size.x + square_projection_on_rect_x - epsilon {
            return false;
        }
        if translation_z >= rect_half_size.y + square_projection_on_rect_z - epsilon {
            return false;
        }

        let rect_projection_on_world_x = rect_half_size.x * rect_axis_x.x.abs()
            + rect_half_size.y * rect_axis_z.x.abs();
        let rect_projection_on_world_z = rect_half_size.x * rect_axis_x.y.abs()
            + rect_half_size.y * rect_axis_z.y.abs();
        if delta.x.abs() >= rect_projection_on_world_x + subtile_half_size.x - epsilon {
            return false;
        }
        if delta.y.abs() >= rect_projection_on_world_z + subtile_half_size.y - epsilon {
            return false;
        }

        true
    }

    fn grid_to_chunk_local(grid_x: i32, grid_z: i32) -> (GridChunkPos, u32, u32) {
        let chunk_size = CHUNK_GRID_SUBDIVISIONS as i32;
        let chunk_x = grid_x.div_euclid(chunk_size);
        let chunk_z = grid_z.div_euclid(chunk_size);
        let local_x = grid_x.rem_euclid(chunk_size) as u32;
        let local_z = grid_z.rem_euclid(chunk_size) as u32;

        (GridChunkPos::new(chunk_x, chunk_z), local_x, local_z)
    }
}

impl GridEntityRotation {
    fn as_radians(self) -> f32 {
        match self {
            GridEntityRotation::R0 => 0.0,
            GridEntityRotation::R45 => std::f32::consts::FRAC_PI_4,
            GridEntityRotation::R90 => std::f32::consts::FRAC_PI_2,
            GridEntityRotation::R135 => std::f32::consts::FRAC_PI_4 * 3.0,
            GridEntityRotation::R180 => std::f32::consts::PI,
            GridEntityRotation::R225 => std::f32::consts::PI + std::f32::consts::FRAC_PI_4,
            GridEntityRotation::R270 => std::f32::consts::PI + std::f32::consts::FRAC_PI_2,
            GridEntityRotation::R315 => std::f32::consts::TAU - std::f32::consts::FRAC_PI_4,
        }
    }
}
