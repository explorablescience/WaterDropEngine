use bevy::prelude::*;
use std::sync::{Arc, RwLock};

/// Generic collider component that can represent different types of colliders.
/// This component requires a [`Transform`] component to be present on the same entity.
#[derive(Component)]
#[require(Transform)]
pub struct Collider {
    pub(crate) data: Arc<RwLock<Box<dyn ColliderShape + Send + Sync>>>
}
impl Collider {
    /// Create a cuboid collider component.
    /// This will also generate a fixed rigid body for the collider, positioned at the collider's transform.
    ///
    /// # Arguments
    /// * `hx` - Half extent along the x-axis.
    /// * `hy` - Half extent along the y-axis.
    /// * `hz` - Half extent along the z-axis.
    pub fn cuboid(hx: f32, hy: f32, hz: f32) -> Self {
        Collider {
            data: Arc::new(RwLock::new(Box::new(CuboidCollider { hx, hy, hz })))
        }
    }

    /// Create a heightfield collider component.
    /// This will also generate a fixed rigid body for the collider, positioned at the collider's transform.
    ///
    /// # Arguments
    /// * `heights` - The height values of the heightfield, stored in a 2D grid (flattened into a 1D vector).
    /// * `scale` - The scale of the heightfield.
    pub fn heightfield(heights: Vec<f32>, scale: [f32; 3]) -> Self {
        Collider {
            data: Arc::new(RwLock::new(Box::new(HeightfieldCollider {
                heights,
                scale
            })))
        }
    }
}

/// Trait for collider shapes.
pub(crate) trait ColliderShape {
    fn build(&self) -> rapier3d::prelude::Collider;
}

pub(crate) struct CuboidCollider {
    /// Half extents
    hx: f32,
    hy: f32,
    hz: f32
}
impl ColliderShape for CuboidCollider {
    fn build(&self) -> rapier3d::prelude::Collider {
        rapier3d::prelude::ColliderBuilder::cuboid(self.hx, self.hy, self.hz).build()
    }
}

pub(crate) struct HeightfieldCollider {
    /// Height values stored in a 2D grid (flattened into a 1D vector).
    heights: Vec<f32>,
    scale: [f32; 3]
}
impl ColliderShape for HeightfieldCollider {
    fn build(&self) -> rapier3d::prelude::Collider {
        // Convert the vector of heights to a rapier3d::nalgebra::Matrix
        let nrows = (self.heights.len() as f32).sqrt() as usize;
        let ncols = nrows;
        let heights_matrix =
            rapier3d::prelude::DMatrix::from_row_slice(nrows, ncols, &self.heights);

        // Build the heightfield collider using the heights matrix and the scale and height multiplier
        rapier3d::prelude::ColliderBuilder::heightfield(
            heights_matrix,
            rapier3d::prelude::Vector::new(self.scale[0], self.scale[1], self.scale[2])
        )
        .build()
    }
}
