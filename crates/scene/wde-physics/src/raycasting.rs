//! Raycasting utilities for physics queries.
//!
//! This module provides types and utilities for casting rays through the physics world
//! to detect collisions and intersections.

use bevy::prelude::*;

pub use rapier3d::prelude::QueryFilter as RayQueryFilter;
use wde_camera::prelude::CameraView;

/// Configuration for ray casting operations.
///
/// Controls how rays interact with colliders during physics queries.
#[derive(Clone)]
pub struct RayCastConfig {
    /// Maximum distance the ray can travel. Use `f32::MAX` for unlimited range.
    pub max_toi: f32,
    /// Whether to treat colliders as solid.
    /// If `true`, rays starting inside a collider will register a hit when exiting.
    /// If `false`, rays ignore colliders they start inside.
    pub solid: bool,
    /// Filter to selectively include/exclude colliders from the raycast.
    /// See `QueryFilter` documentation for filtering options.
    pub filter: RayQueryFilter<'static>,
}
impl Default for RayCastConfig {
    fn default() -> Self {
        RayCastConfig {
            max_toi: f32::MAX,
            solid: true,
            filter: RayQueryFilter::default(),
        }
    }
}

/// A ray in 3D space for physics queries.
///
/// Rays are defined by an origin point and a direction vector. They can be used
/// to perform line-of-sight checks, mouse picking, and other spatial queries.
///
/// # Example
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use wde_physics::prelude::*;
/// // Create a ray pointing downward from (0, 10, 0)
/// let ray = Ray::new(
///     Vec3::new(0.0, 10.0, 0.0),
///     Vec3::new(0.0, -1.0, 0.0),
/// );
/// ```
pub struct Ray(pub(crate) rapier3d::prelude::Ray);
impl Ray {
    /// Create a new ray from an origin and direction.
    /// 
    /// # Arguments
    /// * `origin` - The origin point of the ray.
    /// * `dir` - The direction vector of the ray.
    /// 
    /// # Returns
    /// A new `Ray` in world space.
    pub fn new(origin: Vec3, dir: Vec3) -> Self {
        Ray(rapier3d::prelude::Ray::new(
            rapier3d::prelude::Point::new(origin.x, origin.y, origin.z),
            rapier3d::prelude::Vector::new(dir.x, dir.y, dir.z),
        ))
    }

    /// Create a ray from normalized device coordinates (NDC).
    ///
    /// This is useful for mouse picking and screen-space raycasting. The ray originates
    /// at the camera position and points in the direction corresponding to the given
    /// screen coordinates.
    /// 
    /// # Arguments
    /// * `ndc_pos` - Normalized screen position (0.0 to 1.0 range, where (0,0) is top-left).
    /// * `aspect_ratio` - The viewport aspect ratio (width / height).
    /// * `camera_transform` - The world transform of the camera.
    /// * `camera_view` - The camera's view settings (FOV, near/far planes).
    /// 
    /// # Returns
    /// A new `Ray` originating from the camera and pointing through the screen position.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use bevy::prelude::*;
    /// # use bevy::window::PrimaryWindow;
    /// # use wde_physics::prelude::*;
    /// # use wde_camera::prelude::*;
    /// # fn example(
    /// #     window: Single<&Window, With<PrimaryWindow>>,
    /// #     camera_query: Query<(&Transform, &CameraView), With<Camera>>
    /// # ) {
    /// let cursor_pos = window.cursor_position().unwrap();
    /// let cursor_ndc = cursor_pos / window.size();
    /// let (cam_transform, cam_view) = camera_query.single();
    /// let aspect = window.size().x / window.size().y;
    ///
    /// let ray = Ray::from_ndc(cursor_ndc, aspect, cam_transform, cam_view);
    /// # }
    /// ```
    pub fn from_ndc(
        ndc_pos: Vec2,
        aspect_ratio: f32,
        camera_transform: &Transform,
        camera_view: &CameraView
    ) -> Self {
        // Convert ndc to world direction
        let ndc_pos = ndc_pos * 2.0 - Vec2::ONE;
        let ndc_pos = Vec2::new(ndc_pos.x, -ndc_pos.y); // Invert Y for NDC
        let dir = camera_view.ndc_to_world(ndc_pos, camera_transform, aspect_ratio);

        // Start the ray at the camera position
        let origin = camera_transform.translation;

        // Return the ray
        Ray::new(origin, dir)
    }

    /// Get a point along the ray at time of impact (toi).
    /// 
    /// # Arguments
    /// * `toi` - The time of impact along the ray.
    /// 
    /// # Returns
    /// A point in world space at the specified time of impact.
    pub fn point_at(&self, toi: f32) -> Vec3 {
        let point = self.0.point_at(toi);
        Vec3::new(point.x, point.y, point.z)
    }
}
