use bevy::prelude::*;

use super::TransformUniform;

/// Tag that list the current active camera.
#[derive(Component, Default, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct ActiveCamera;

/// Camera view component with field of view, aspect ratio, near and far planes.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct CameraView {
    pub fov: f32,
    pub znear: f32,
    pub zfar: f32,
}
impl Default for CameraView {
    fn default() -> Self {
        Self {
            fov: 60.0,
            znear: 0.1,
            zfar: 1000.0,
        }
    }
}

/// Camera is defined by a position and a view.
#[derive(Component, Default, Clone, Debug, Reflect)]
#[reflect(Component)]
#[require(Transform, CameraView)]
pub struct Camera;

/// Camera uniform buffer.
#[repr(C)]
#[derive(Resource, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    // From world to NDC coordinates
    pub world_to_ndc: [[f32; 4]; 4],
    // From NDC to world coordinates
    pub ndc_to_world: [[f32; 4]; 4],
    // Camera position
    pub position: [f32; 4]
}

impl CameraUniform {
    /// Create a new camera uniform buffer.
    /// 
    /// # Arguments
    /// 
    /// * `camera` - The camera component.
    /// * `transform` - The transform component.
    /// * `aspect_ratio` - The aspect ratio of the screen.
    ///
    /// # Returns
    /// 
    /// The camera uniform buffer.
    pub fn new(transform: &Transform, camera_view: &CameraView, aspect_ratio: f32) -> Self {
        let world_to_ndc = Self::get_world_to_ndc(transform, camera_view, aspect_ratio).to_cols_array_2d();
        let ndc_to_world = Self::get_ndc_to_world(transform, camera_view, aspect_ratio).to_cols_array_2d();

        Self {
            world_to_ndc,
            ndc_to_world,
            position: [transform.translation.x, transform.translation.y, transform.translation.z, 1.0]
        }
    }

    /// Get the world to ndc matrix.
    /// 
    /// # Arguments
    /// 
    /// * `camera` - The camera component.
    /// * `transform` - The transform component.
    /// * `aspect_ratio` - The aspect ratio of the screen.
    /// 
    /// # Returns
    /// 
    /// The world to screen (NDC) matrix ((openGL to WGPU) * projection * view).
    #[inline]
    fn get_world_to_ndc(transform: &Transform, camera_view: &CameraView, aspect_ratio: f32) -> Mat4 {
        // World to camera
        let view = TransformUniform::transform_world_to_obj(transform);

        // Projection from camera to NDC
        let proj = Mat4::perspective_rh(
            camera_view.fov.to_radians(), aspect_ratio,
            camera_view.znear, camera_view.zfar
        );
        proj * view
    }

    /// Get the ndc to world matrix.
    /// 
    /// # Arguments
    /// 
    /// * `camera` - The camera component.
    /// * `transform` - The transform component.
    /// * `aspect_ratio` - The aspect ratio of the screen.
    /// 
    /// # Returns
    /// 
    /// The screen (NDC) to world matrix (inverse(projection * view) * inverse(openGL to WGPU)).
    #[inline]
    fn get_ndc_to_world(transform: &Transform, camera_view: &CameraView, aspect_ratio: f32) -> Mat4 {
        Self::get_world_to_ndc(transform, camera_view, aspect_ratio).inverse()
    }
}
