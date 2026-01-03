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
}
