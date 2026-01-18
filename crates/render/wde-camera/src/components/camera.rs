use bevy::prelude::*;
use wde_renderer::prelude::TransformUniform;

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
impl CameraView {
    /// Get the aspect ratio from the window size.
    pub fn aspect_ratio(&self, window_size: Vec2) -> f32 {
        window_size.x / window_size.y
    }

    /// Convert a 2D ndc position to a 3D world direction.
    /// 
    /// # Arguments
    /// 
    /// * `ndc_pos` - The 2D ndc position.
    /// * `transform` - The camera transform.
    /// * `aspect_ratio` - The aspect ratio of the viewport.
    /// 
    /// # Returns
    /// 
    /// The 3D world direction.
    pub fn ndc_to_world(&self, ndc_pos: Vec2, transform: &Transform, aspect_ratio: f32) -> Vec3 {
        let proj = Mat4::perspective_rh(
            self.fov.to_radians(), aspect_ratio,
            self.znear, self.zfar
        );
        let view = TransformUniform::transform_world_to_obj(transform);
        let inv_vp = (proj * view).inverse();

        let ndc_near = Vec4::new(ndc_pos.x, ndc_pos.y, -1.0, 1.0);
        let ndc_far = Vec4::new(ndc_pos.x, ndc_pos.y, 1.0, 1.0);

        let world_near = inv_vp * ndc_near;
        let world_far = inv_vp * ndc_far;

        let world_near = world_near.xyz() / world_near.w;
        let world_far = world_far.xyz() / world_far.w;

        (world_far - world_near).normalize()
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
pub(crate) struct CameraUniform {
    // From world to view coordinates
    pub world_to_view: [[f32; 4]; 4],
    // From view to ndc coordinates
    pub view_to_ndc: [[f32; 4]; 4],
    // From ndc to view coordinates
    pub ndc_to_view: [[f32; 4]; 4],
    // From view to world coordinates
    pub view_to_world: [[f32; 4]; 4],
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
        let world_to_view = Self::get_world_to_view(transform).to_cols_array_2d();
        let view_to_ndc = Self::get_view_to_ndc(camera_view, aspect_ratio).to_cols_array_2d();
        let ndc_to_view = Mat4::from_cols_array_2d(&view_to_ndc).inverse().to_cols_array_2d();
        let view_to_world = Mat4::from_cols_array_2d(&world_to_view).inverse().to_cols_array_2d();

        Self {
            world_to_view,
            view_to_ndc,
            ndc_to_view,
            view_to_world,
            position: [transform.translation.x, transform.translation.y, transform.translation.z, 1.0]
        }
    }

    /// Get the world to ndc matrix.
    /// 
    /// # Arguments
    /// 
    /// * `transform` - The transform component.
    /// * `camera_view` - The camera view component.
    /// * `aspect_ratio` - The aspect ratio of the screen.
    /// 
    /// # Returns
    /// 
    /// The world to screen (NDC) matrix ((openGL to WGPU) * projection * view).
    #[inline]
    #[allow(dead_code)]
    fn get_world_to_ndc(transform: &Transform, camera_view: &CameraView, aspect_ratio: f32) -> Mat4 {
        let view = Self::get_world_to_view(transform);
        let proj = Self::get_view_to_ndc(camera_view, aspect_ratio);
        proj * view
    }

    /// Get the world to view matrix.
    /// 
    /// # Arguments
    /// 
    /// * `transform` - The transform component.
    /// 
    /// # Returns
    /// 
    /// The world to view matrix.
    #[inline]
    fn get_world_to_view(transform: &Transform) -> Mat4 {
        // World to camera
        TransformUniform::transform_world_to_obj(transform)
    }

    /// Get the view to ndc matrix.
    /// 
    /// # Arguments
    /// 
    /// * `camera_view` - The camera view component.
    /// * `aspect_ratio` - The aspect ratio of the screen.
    /// 
    /// # Returns
    /// 
    /// The view to screen (NDC) matrix (projection * openGL to WGPU).
    #[inline]
    fn get_view_to_ndc(camera_view: &CameraView, aspect_ratio: f32) -> Mat4 {
        // Projection from camera to NDC
        Mat4::perspective_rh(
            camera_view.fov.to_radians(), aspect_ratio,
            camera_view.znear, camera_view.zfar
        )
    }
}
