use bevy::prelude::*;
use rapier3d::prelude::InteractionGroups;
use std::sync::{Arc, RwLock};

pub use rapier3d::prelude::Group as ColliderGroup;

/// Generic collider component that can represent different types of colliders.
/// This component requires a [`Transform`] component to be present on the same entity.
#[derive(Component)]
#[require(Transform)]
pub struct Collider {
    pub(crate) data: Arc<RwLock<Box<dyn ColliderShape + Send + Sync>>>
}
impl Collider {
    /// Create a new collider from a shape that implements the [`ColliderShape`] trait.
    /// See the documentation of the trait for available shapes and how to implement custom ones.
    pub fn from(shape: impl ColliderShape + Send + Sync + 'static) -> Self {
        Collider {
            data: Arc::new(RwLock::new(Box::new(shape)))
        }
    }
}

/// Trait for collider shapes.
pub trait ColliderShape {
    fn build(&self) -> rapier3d::prelude::Collider;
}

/// A simple box collider shape.
/// The collider is defined by its half extents along each axis and an optional collision group.
pub struct BoxCollider {
    /// Half extents
    hx: f32,
    hy: f32,
    hz: f32,

    /// Optional membership in a collision group. If `None`, the collider will be in the default group.
    group: Option<ColliderGroup>
}
impl BoxCollider {
    /// Create a new box collider with the specified extent and collision group.
    /// The extent is the full size of the box along each axis (not half extents).
    pub fn new(extent: Vec3) -> Self {
        BoxCollider {
            hx: extent.x / 2.0,
            hy: extent.y / 2.0,
            hz: extent.z / 2.0,
            group: None
        }
    }

    /// Set the collision group for the box collider.
    pub fn with_group(mut self, group: ColliderGroup) -> Self {
        self.group = Some(group);
        self
    }
}
impl ColliderShape for BoxCollider {
    fn build(&self) -> rapier3d::prelude::Collider {
        let mut collider = rapier3d::prelude::ColliderBuilder::cuboid(self.hx, self.hy, self.hz);
        if let Some(group) = self.group {
            collider =
                collider.collision_groups(InteractionGroups::new(group, ColliderGroup::all()));
        }
        collider.build()
    }
}

/// A heightfield collider shape.
/// The collider is defined by a grid of height values and a scale factor.
pub struct HeightfieldCollider {
    /// Height values stored in a 2D grid (flattened into a 1D vector).
    heights: Vec<f32>,
    scale: [f32; 3],

    /// Optional membership in a collision group. If `None`, the collider will be in the default group.
    group: Option<ColliderGroup>
}
impl HeightfieldCollider {
    /// Create a new heightfield collider from a vector of heights and a scale factor.
    /// The heights vector should represent a 2D grid of height values, flattened into a 1D vector (row-major order).
    /// The scale factor defines the size of each grid cell in world units (x and z) and the height multiplier (y).
    pub fn from_heights(heights: Vec<f32>, scale: [f32; 3]) -> Self {
        HeightfieldCollider {
            heights,
            scale,
            group: None
        }
    }

    /// Create a new heightfield collider from a heightmap image.
    /// The image data should be in RGBA format, where the red channel represents the height values (grayscale image). The scale factor defines the size of each grid cell in world units (x and z) and the height multiplier (y).
    pub fn from_heightmap(data: &[u8], scale: [f32; 3]) -> Self {
        // Convert the image data to a vector of heights (assuming the image is grayscale)
        let heights = data
            .chunks(4)
            .map(|pixel| {
                // Take the red channel as the height value (since it's a grayscale image, all channels should be the same)
                pixel[0] as f32 / 255.0
            })
            .collect();

        HeightfieldCollider {
            heights,
            scale,
            group: None
        }
    }

    /// Set the collision group for the heightfield collider.
    pub fn with_group(mut self, group: ColliderGroup) -> Self {
        self.group = Some(group);
        self
    }
}
impl ColliderShape for HeightfieldCollider {
    fn build(&self) -> rapier3d::prelude::Collider {
        // Convert the vector of heights to a rapier3d::nalgebra::Matrix
        let nrows = (self.heights.len() as f32).sqrt() as usize;
        let ncols = nrows;
        let heights_matrix =
            rapier3d::prelude::DMatrix::from_row_slice(nrows, ncols, &self.heights);

        // Build the heightfield collider using the heights matrix and the scale and height multiplier
        let mut heightfield = rapier3d::prelude::ColliderBuilder::heightfield(
            heights_matrix,
            rapier3d::prelude::Vector::new(self.scale[0], self.scale[1], self.scale[2])
        );
        if let Some(group) = self.group {
            heightfield =
                heightfield.collision_groups(InteractionGroups::new(group, ColliderGroup::all()));
        }
        heightfield.build()
    }
}
