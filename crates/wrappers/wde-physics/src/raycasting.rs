use bevy::prelude::*;

pub use rapier3d::prelude::QueryFilter as RayQueryFilter;
use rapier3d::prelude::{Group as ColliderGroup, InteractionGroups};
use wde_camera::prelude::CameraView;

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
    /// See [`RayQueryFilter`] documentation for filtering options.
    pub filter: RayQueryFilter<'static>
}
impl Default for RayCastConfig {
    fn default() -> Self {
        RayCastConfig {
            max_toi: f32::MAX,
            solid: true,
            filter: RayQueryFilter::default()
        }
    }
}
impl RayCastConfig {
    /// Only include colliders that are part of the specified collision group(s).
    pub fn with_filter(filter: ColliderGroup) -> Self {
        RayCastConfig {
            filter: RayQueryFilter::new()
                .groups(InteractionGroups::new(ColliderGroup::all(), filter)),
            ..Default::default()
        }
    }
}

/// A ray in 3D space for physics queries.
/// It can be casted using methods on [`crate::PhysicsWorld`].
/// See [crate] for examples of raycasting usage.
pub struct Ray(pub(crate) rapier3d::prelude::Ray);
impl Ray {
    /// Create a new ray from an origin and direction.
    pub fn new(origin: Vec3, dir: Vec3) -> Self {
        Ray(rapier3d::prelude::Ray::new(
            rapier3d::prelude::Point::new(origin.x, origin.y, origin.z),
            rapier3d::prelude::Vector::new(dir.x, dir.y, dir.z)
        ))
    }

    /// Create a ray from normalized device coordinates (NDC).
    /// This is useful for casting rays from the camera through the screen (e.g. for mouse picking).
    pub fn from_ndc(
        ndc_pos: Vec2,
        aspect_ratio: f32,
        camera_transform: &GlobalTransform,
        camera_view: &CameraView
    ) -> Self {
        // Convert ndc to world direction
        let ndc_pos = ndc_pos * 2.0 - Vec2::ONE;
        let ndc_pos = Vec2::new(ndc_pos.x, -ndc_pos.y); // Invert Y for NDC
        let dir = camera_view.ndc_to_world(ndc_pos, camera_transform, aspect_ratio);

        // Start the ray at the camera position
        let origin = camera_transform.translation();

        // Return the ray
        Ray::new(origin, dir)
    }

    /// Get a point along the ray at time of impact (toi).
    pub fn point_at(&self, toi: f32) -> Vec3 {
        let point = self.0.point_at(toi);
        Vec3::new(point.x, point.y, point.z)
    }
}
