use bevy::prelude::*;

use crate::core::grid::{GridChunkPos, GridLocalPos};

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
    pub fn get_occupied_tiles(&self) -> Vec<(GridChunkPos, GridLocalPos)> {
        Vec::new()
    }
}
