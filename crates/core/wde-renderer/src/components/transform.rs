use bevy::prelude::*;

/// Define the transform uniform buffer aligned to 16 bytes for the GPU.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TransformUniform {
    /// From object to world space.
    pub object_to_world: [[f32; 4]; 4]
}

impl TransformUniform {
    /// Create a new transform uniform.
    /// 
    /// # Arguments
    /// 
    /// * `transform` - The transform component.
    /// 
    /// # Returns
    /// 
    /// The transform uniform.
    pub fn new(transform: &Transform) -> Self {
        Self {
            object_to_world: Self::transform_obj_to_world(transform).to_cols_array_2d()
        }
    }
}

impl TransformUniform {
    /// Get the matrix transform from object space to world space.
    /// 
    /// # Arguments
    /// 
    /// * `transform` - The transform component.
    /// 
    /// # Returns
    /// 
    /// The matrix transform from object to world space (translate * rotate * scale).
    #[inline]
    pub fn transform_obj_to_world(transform: &Transform) -> Mat4 {
        Mat4::from_scale_rotation_translation(transform.scale, transform.rotation, transform.translation)
    }

    /// Get the matrix transform from world space to object space.
    /// 
    /// # Arguments
    /// 
    /// * `transform` - The transform component.
    /// 
    /// # Returns
    /// 
    /// The matrix transform from world to object space (translate * rotate * scale)^(-1).
    #[inline]
    pub fn transform_world_to_obj(transform: &Transform) -> Mat4 {
        Mat4::from_scale_rotation_translation(transform.scale, transform.rotation, transform.translation).inverse()
    }

    /// Get the forward vector (z axis) that the object is facing.
    /// 
    /// # Arguments
    /// 
    /// * `transform` - The transform component.
    #[inline]
    pub fn forward(transform: Transform) -> Vec3 {
        transform.rotation * Vec3 { x: 0.0, y: 0.0, z: 1.0 }
    }

    /// Get the right vector (x axis) that the object is facing.
    /// 
    /// # Arguments
    /// 
    /// * `transform` - The transform component.
    #[inline]
    pub fn right(transform: Transform) -> Vec3 {
        transform.rotation * Vec3 { x: 1.0, y: 0.0, z: 0.0 }
    }

    /// Get the up vector (y axis) that the object is facing.
    /// 
    /// # Arguments
    /// 
    /// * `transform` - The transform component.
    #[inline]
    pub fn up(transform: Transform) -> Vec3 {
        transform.rotation * Vec3 { x: 0.0, y: 1.0, z: 0.0 }
    }
}
