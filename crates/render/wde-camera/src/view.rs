use bevy::prelude::*;

use crate::prelude::TransformUniform;

/// Camera view component with field of view, aspect ratio, near and far planes.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct CameraView {
    pub fov: f32,
    pub znear: f32,
    pub zfar: f32
}
impl Default for CameraView {
    fn default() -> Self {
        Self {
            fov: 60.0,
            znear: 0.1,
            zfar: 10000.0
        }
    }
}
impl CameraView {
    /// Get the aspect ratio from the window size.
    pub fn aspect_ratio(&self, window_size: Vec2) -> f32 {
        window_size.x / window_size.y
    }

    /// World to NDC matrix of this view from a camera at `transform`.
    pub fn world_to_ndc(&self, transform: &GlobalTransform, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov.to_radians(), aspect_ratio, self.znear, self.zfar)
            * TransformUniform::transform_world_to_obj(transform)
    }

    /// Convert a 2D ndc position to a 3D world direction.
    pub fn ndc_to_world(
        &self,
        ndc_pos: Vec2,
        transform: &GlobalTransform,
        aspect_ratio: f32
    ) -> Vec3 {
        let inv_vp = self.world_to_ndc(transform, aspect_ratio).inverse();

        let ndc_near = Vec4::new(ndc_pos.x, ndc_pos.y, -1.0, 1.0);
        let ndc_far = Vec4::new(ndc_pos.x, ndc_pos.y, 1.0, 1.0);

        let world_near = inv_vp * ndc_near;
        let world_far = inv_vp * ndc_far;

        let world_near = world_near.xyz() / world_near.w;
        let world_far = world_far.xyz() / world_far.w;

        (world_far - world_near).normalize()
    }
}

/// Camera uniform buffer.
#[repr(C)]
#[derive(Resource, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct CameraUniform {
    // From world to view coordinates
    world_to_view: [[f32; 4]; 4],
    // From view to ndc coordinates
    pub view_to_ndc: [[f32; 4]; 4],
    // From ndc to view coordinates
    pub ndc_to_view: [[f32; 4]; 4],
    // From view to world coordinates
    pub view_to_world: [[f32; 4]; 4],
    // Camera position
    pub position: [f32; 4],
    // From world to ndc coordinates, without the jitter
    pub world_to_ndc_unjittered: [[f32; 4]; 4],
    // `world_to_ndc_unjittered` of the previous frame
    pub prev_world_to_ndc: [[f32; 4]; 4],
    // xy = offset of the projection in NDC
    pub jitter: [f32; 4]
}
impl CameraUniform {
    /// Create a new camera uniform buffer. The projection is offset by `jitter` in NDC, and `prev_world_to_ndc` is
    /// the unjittered world to ndc matrix of the previous frame.
    pub fn new(
        transform: &GlobalTransform,
        camera_view: &CameraView,
        aspect_ratio: f32,
        jitter: Vec2,
        prev_world_to_ndc: Mat4
    ) -> Self {
        let world_to_view = Self::get_world_to_view(transform);
        let projection = Self::get_view_to_ndc(camera_view, aspect_ratio);
        let view_to_ndc = Mat4::from_translation(jitter.extend(0.0)) * projection;

        Self {
            world_to_view: world_to_view.to_cols_array_2d(),
            view_to_ndc: view_to_ndc.to_cols_array_2d(),
            ndc_to_view: view_to_ndc.inverse().to_cols_array_2d(),
            view_to_world: world_to_view.inverse().to_cols_array_2d(),
            position: [
                transform.translation().x,
                transform.translation().y,
                transform.translation().z,
                1.0
            ],
            world_to_ndc_unjittered: (projection * world_to_view).to_cols_array_2d(),
            prev_world_to_ndc: prev_world_to_ndc.to_cols_array_2d(),
            jitter: [jitter.x, jitter.y, 0.0, 0.0]
        }
    }

    /// Get the world to view matrix.
    #[inline]
    fn get_world_to_view(transform: &GlobalTransform) -> Mat4 {
        // World to camera
        TransformUniform::transform_world_to_obj(transform)
    }

    /// Get the view to ndc matrix.
    #[inline]
    fn get_view_to_ndc(camera_view: &CameraView, aspect_ratio: f32) -> Mat4 {
        // Projection from camera to NDC
        Mat4::perspective_rh(
            camera_view.fov.to_radians(),
            aspect_ratio,
            camera_view.znear,
            camera_view.zfar
        )
    }
}
