use bevy::prelude::*;

use crate::{
    core::{
        entries::PlacementConfigEntry,
        grid::{GridTilePos, TILE_SIZE}
    },
    prelude::Grid
};

/// Local rotation of an entity on the grid around its center, in 90 degree increments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum GridRotation {
    #[default]
    R0,
    R90,
    R180,
    R270
}
impl GridRotation {
    /// Gives the rotation resulting from the given rotation.
    pub fn rotation(self) -> f32 {
        match self {
            GridRotation::R0 => 0.0,
            GridRotation::R90 => std::f32::consts::FRAC_PI_2,
            GridRotation::R180 => std::f32::consts::PI,
            GridRotation::R270 => 3.0 * std::f32::consts::FRAC_PI_2
        }
    }
}

/// Describes an entity that has been placed on the grid.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct GridEntity {
    entry: PlacementConfigEntry,
    center: Vec2,
    rotation: GridRotation,
    bbox: (Vec2, Vec2), // (bottom left, top_right)
    footprint: Vec<GridTilePos>
}
impl GridEntity {
    /// Creates a new GridEntity with the given center position, rotation, and placement configuration entry.
    ///
    /// # Arguments
    /// * `center` - The center position of the entity in world coordinates. Note that this is not necessarily the center of the footprint, but the position used to place the entity on the grid. It will be adjusted to the nearest grid tile.
    /// * `rotation` - The rotation of the entity on the grid, in 90 degree increments. This will affect the footprint of the entity.
    /// * `entry` - The placement configuration entry that describes the entity, including its extent and anchors.
    pub fn new(center: Vec2, rotation: GridRotation, entry: PlacementConfigEntry) -> Self {
        let (center, bbox, footprint) = compute_footprint(center, entry.extent, rotation);
        GridEntity {
            entry,
            center,
            rotation,
            bbox,
            footprint
        }
    }
    /// Get the center position of this entity in world coordinates.
    pub fn center(&self) -> Vec2 {
        self.center
    }
    /// Get the rotation of this entity on the grid.
    pub fn rotation(&self) -> GridRotation {
        self.rotation
    }
    /// Gets the bounding box of this entity in world coordinates (bottom left, top right).
    pub fn bbox(&self) -> (Vec2, Vec2) {
        self.bbox
    }
    /// Gets the list of grid tiles that are occupied by this entity footprint.
    pub fn footprint(&self) -> &Vec<GridTilePos> {
        &self.footprint
    }
    /// Gets a reference to the placement configuration entry that describes this entity.
    pub fn entry(&self) -> &PlacementConfigEntry {
        &self.entry
    }
}

fn compute_footprint(
    center: Vec2,
    extent: UVec2,
    rotation: GridRotation
) -> (Vec2, (Vec2, Vec2), Vec<GridTilePos>) {
    // Change extent if rotated
    let extent = match rotation {
        GridRotation::R0 | GridRotation::R180 => extent,
        GridRotation::R90 | GridRotation::R270 => UVec2::new(extent.y, extent.x)
    };

    // Add an offset to start from the center of the object
    let offset_to_center_object =
        Vec2::new(extent.x as f32, extent.y as f32) * TILE_SIZE / 2.0 - TILE_SIZE / 2.0;

    // Compute the footprint
    let mut footprint = Vec::new();
    for x in 0..extent.x {
        for z in 0..extent.y {
            let local_pos =
                center - offset_to_center_object + Vec2::new(x as f32, z as f32) * TILE_SIZE;
            let (chunk_pos, local_pos) = Grid::get_nearest_tile(local_pos);
            footprint.push((chunk_pos, (local_pos.0, local_pos.1)));
        }
    }

    // Compute bbox
    let bottom_left_pos = center - offset_to_center_object
        + Vec2::new(extent.x as f32 - 1.0, extent.y as f32 - 1.0) * TILE_SIZE;
    let bottom_left_pos = Grid::get_tile_world_pos(Grid::get_nearest_tile(bottom_left_pos));
    let top_right_pos = center - offset_to_center_object;
    let top_right_pos = Grid::get_tile_world_pos(Grid::get_nearest_tile(top_right_pos));
    let center = (bottom_left_pos + top_right_pos) / 2.0;

    (center, (bottom_left_pos, top_right_pos), footprint)
}
