use bevy::prelude::*;

/// Define the transform uniform buffer aligned to 16 bytes for the GPU.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TransformUniform {
    /// From object to world space.
    object_to_world: [[f32; 4]; 4]
}
impl TransformUniform {
    /// Create a new transform uniform.
    #[inline]
    pub fn new(transform: &GlobalTransform) -> Self {
        Self {
            object_to_world: Self::transform_obj_to_world(transform).to_cols_array_2d()
        }
    }

    /// Get the matrix transform from object space to world space (translate * rotate * scale).
    #[inline]
    pub fn transform_obj_to_world(transform: &GlobalTransform) -> Mat4 {
        transform.to_matrix()
    }
    /// Get the matrix transform from world space to object space (translate * rotate * scale)^(-1).
    #[inline]
    pub fn transform_world_to_obj(transform: &GlobalTransform) -> Mat4 {
        transform.to_matrix().inverse()
    }

    /// Get the forward vector (z axis) that the object is facing.
    #[inline]
    pub fn forward(transform: &GlobalTransform) -> Dir3 {
        transform.forward()
    }
    /// Get the right vector (x axis) that the object is facing.
    #[inline]
    pub fn right(transform: &GlobalTransform) -> Dir3 {
        transform.right()
    }
    /// Get the up vector (y axis) that the object is facing.
    #[inline]
    pub fn up(transform: &GlobalTransform) -> Dir3 {
        transform.up()
    }
}
