//! Collider components for physics simulation.
//!
//! This module provides components that can be attached to entities to give them
//! physical collision shapes in the physics world.

use bevy::prelude::*;
use std::sync::{Arc, RwLock};

/// Trait for collider shapes.
///
/// This trait is implemented by different collider shape types to provide
/// a unified interface for building Rapier colliders.
pub(crate) trait ColliderShape {
    /// Build the rapier collider from the shape.
    /// 
    /// # Returns
    /// The rapier collider.
    fn build(&self) -> rapier3d::prelude::Collider;
}

/// Cuboid (box) collider shape.
///
/// Represents a rectangular box collider with half-extents along each axis.
pub(crate) struct CuboidCollider {
    /// Half extent along the x-axis
    hx: f32,
    /// Half extent along the y-axis
    hy: f32,
    /// Half extent along the z-axis
    hz: f32,
}
impl ColliderShape for CuboidCollider {
    fn build(&self) -> rapier3d::prelude::Collider {
        rapier3d::prelude::ColliderBuilder::cuboid(self.hx, self.hy, self.hz).build()
    }
}

/// Heightfield collider shape.
/// 
/// Represents a heightfield collider, which is a grid of height values used to create a terrain-like collision shape.
pub(crate) struct HeightfieldCollider {
    /// The height values of the heightfield, stored in a 2D grid (flattened into a 1D vector).
    heights: Vec<f32>,
    /// The scale of the heightfield
    scale: [f32; 3],
}
impl ColliderShape for HeightfieldCollider {
    fn build(&self) -> rapier3d::prelude::Collider {
        // Convert the vector of heights to a rapier3d::nalgebra::Matrix
        let nrows = (self.heights.len() as f32).sqrt() as usize;
        let ncols = nrows;
        let heights_matrix = rapier3d::prelude::DMatrix::from_row_slice(nrows, ncols, &self.heights);

        // Build the heightfield collider using the heights matrix and the scale and height multiplier
        rapier3d::prelude::ColliderBuilder::heightfield(
            heights_matrix,
            rapier3d::prelude::Vector::new(self.scale[0], self.scale[1], self.scale[2])
        ).build()
    }
}

/// A physics collider component.
///
/// Attach this component to an entity to give it a collision shape in the physics world.
/// The collider will automatically be created at the entity's transform position and
/// updated when the transform changes.
///
/// # Example
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use wde_physics::prelude::*;
/// # fn example(mut commands: Commands) {
/// // Create a cube collider
/// commands.spawn((
///     Transform::from_xyz(0.0, 5.0, 0.0),
///     Collider::cuboid(1.0, 1.0, 1.0),
/// ));
/// 
/// // Create a heightfield collider
/// let heights = vec![0.0; 16 * 16]; // A flat heightfield with 16x16 samples
/// commands.spawn((
///     Transform::from_xyz(0.0, 0.0, 0.0),
///     Collider::heightfield(heights, 1.0, 10.0),
/// ));
/// # }
/// ```
#[derive(Component)]
#[require(Transform)]
pub struct Collider {
    /// Internal collider shape data
    pub(crate) data: Arc<RwLock<Box<dyn ColliderShape + Send + Sync>>>,
}
impl Collider {
    /// Create a cuboid collider component.
    /// This will also generate a fixed rigid body for the collider, positioned at the collider's transform.
    /// 
    /// # Arguments
    /// * `hx` - Half extent along the x-axis.
    /// * `hy` - Half extent along the y-axis.
    /// * `hz` - Half extent along the z-axis.
    /// 
    /// # Returns
    /// A new `Collider` component representing a cuboid collider.
    pub fn cuboid(hx: f32, hy: f32, hz: f32) -> Self {
        Collider {
            data: Arc::new(RwLock::new(Box::new(CuboidCollider { hx, hy, hz }))),
        }
    }

    /// Create a heightfield collider component.
    /// This will also generate a fixed rigid body for the collider, positioned at the collider's transform.
    /// 
    /// # Arguments
    /// * `heights` - The height values of the heightfield, stored in a 2D grid (flattened into a 1D vector).
    /// * `scale` - The scale of the heightfield.
    /// 
    /// # Returns
    /// A new `Collider` component representing a heightfield collider.
    pub fn heightfield(heights: Vec<f32>, scale: [f32; 3]) -> Self {
        Collider {
            data: Arc::new(RwLock::new(Box::new(HeightfieldCollider {
                heights,
                scale,
            }))),
        }
    }
}
