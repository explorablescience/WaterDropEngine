use bevy::prelude::*;

use crate::{core::grid::GridPos, prelude::{Grid, GridLocalDir}};

/// Local rotation of an entity on the grid around its center, in 90 degree increments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
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

/// Describes the area occupied by an entity on the grid.
#[derive(Component, Clone, Debug)]
pub struct GridEntity {
    center: Vec2,
    bbox: (Vec2, Vec2), // (bottom left, top_right)
    footprint: Vec<GridPos>
}
impl GridEntity {
    pub fn new(center: Vec2, extent: UVec2, rotation: GridRotation) -> Self {
        let (center, bbox, footprint) = compute_footprint(center, extent, rotation);
        GridEntity { center, bbox, footprint }
    }
    /// Gets the center tile position of this entity.
    /// Note that this is not necessarily the center of the footprint, but the position used to place the entity on the grid.
    pub fn center(&self) -> Vec2 {
        self.center
    }
    /// Gets the bounding box of this entity in world coordinates (bottom left, top right).
    pub fn bbox(&self) -> (Vec2, Vec2) {
        self.bbox
    }
    /// Gets the list of grid tiles that are occupied by this entity footprint.
    pub fn footprint(&self) -> &Vec<GridPos> {
        &self.footprint
    }
}

// For non-rotated entities, the footprint is a simple rectangle around the center
fn compute_footprint(center: Vec2, extent: UVec2, rotation: GridRotation) -> (Vec2, (Vec2, Vec2), Vec<GridPos>) {
    let ts = Grid::tile_size();

    // Change extent if rotated
    let extent = match rotation {
        GridRotation::R0 | GridRotation::R180 => extent,
        GridRotation::R90 | GridRotation::R270 => UVec2::new(extent.y, extent.x)
    };

    // Add an offset to start from the center of the object
    let offset_to_center_object = Vec2::new(extent.x as f32, extent.y as f32) * ts / 2.0 - ts / 2.0;

    // Compute the footprint
    let mut footprint = Vec::new();
    let dir_list = [GridLocalDir::North, GridLocalDir::East, GridLocalDir::West, GridLocalDir::South];
    for x in 0..extent.x {
        for z in 0..extent.y {
            let local_pos = center - offset_to_center_object + Vec2::new(x as f32, z as f32) * ts;
            let (chunk_pos, local_pos) = Grid::world_to_pos(local_pos);
            for dir in dir_list {
                let local_pos_s = (local_pos.0, local_pos.1, dir);
                footprint.push((chunk_pos, local_pos_s));
            }
        }
    }

    // Compute bbox
    let bottom_left_pos = center - offset_to_center_object + Vec2::new(extent.x as f32 - 1.0, extent.y as f32 - 1.0) * ts;
    let (bottom_chunk_pos, bottom_local_pos) = Grid::world_to_pos(bottom_left_pos);
    let bottom_left_pos = Grid::pos_to_world_tile_center(bottom_chunk_pos, (bottom_local_pos.0, bottom_local_pos.1));
    let top_right_pos = center - offset_to_center_object;
    let (top_chunk_pos, top_local_pos) = Grid::world_to_pos(top_right_pos);
    let top_right_pos = Grid::pos_to_world_tile_center(top_chunk_pos, (top_local_pos.0, top_local_pos.1));
    let center = (bottom_left_pos + top_right_pos) / 2.0;

    (center, (bottom_left_pos, top_right_pos), footprint)
}
