use bevy::prelude::*;

use crate::core::grid::{GridChunkPos, GridLocalDir, GridLocalPos, CHUNK_GRID_SUBDIVISIONS};

/// Local rotation of an entity on the grid around its center, in 90 degree increments.
#[derive(Clone, Copy, Debug, Default)]
pub enum GridEntityRotation {
    #[default]
    R0,
    R90,
    R180,
    R270,
}

/// Describes the area occupied by an entity on the grid.
#[derive(Component, Clone, Debug)]
pub struct GridEntity {
    /// The center position of the entity in world coordinates.
    pub center: Vec3,
    /// The size of the entity's footprint on the grid (width and depth).
    /// Setting a size of (sx, sy) means that the entity occupies the area from (center.x - sx/2, center.z - sy/2) to (center.x + sx/2, center.z + sy/2).
    pub size: Vec2,
    /// Local rotation of the footprint around its center.
    pub rotation: GridEntityRotation,
}
impl GridEntity {
    /// Gets the list of grid tiles that are occupied by this entity footprint.
    pub(crate) fn get_occupied_tiles(&self) -> Vec<(GridChunkPos, GridLocalPos)> {
        let mut tiles = Vec::new();

        // 90-degree rotations swap width and depth in local space.
        let effective_size = match self.rotation {
            GridEntityRotation::R0 | GridEntityRotation::R180 => self.size,
            GridEntityRotation::R90 | GridEntityRotation::R270 => Vec2::new(self.size.y, self.size.x),
        };

        let half_size = effective_size / 2.0;
        let min_x = self.center.x - half_size.x;
        let max_x = self.center.x + half_size.x;
        let min_z = self.center.z - half_size.y;
        let max_z = self.center.z + half_size.y;

        // Expand by one cell around ceil-bounds so boundary subtiles are tested.
        let grid_min_x = min_x.ceil() as i32 - 1;
        let grid_max_x = max_x.ceil() as i32;
        let grid_min_z = min_z.ceil() as i32 - 1;
        let grid_max_z = max_z.ceil() as i32;

        for x in grid_min_x..=grid_max_x {
            for z in grid_min_z..=grid_max_z {
                for (dir, ox, oz) in [
                    (GridLocalDir::NorthWest, 0.25_f32, 0.25_f32),
                    (GridLocalDir::NorthEast, 0.75_f32, 0.25_f32),
                    (GridLocalDir::SouthWest, 0.25_f32, 0.75_f32),
                    (GridLocalDir::SouthEast, 0.75_f32, 0.75_f32),
                ] {
                    let px = x as f32 + ox;
                    let pz = z as f32 + oz;
                    if px >= min_x && px < max_x && pz >= min_z && pz < max_z {
                        let (chunk_pos, local_x, local_z) = Self::grid_to_chunk_local(x, z);
                        tiles.push((chunk_pos, (local_x, local_z, dir)));
                    }
                }
            }
        }

        tiles
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
