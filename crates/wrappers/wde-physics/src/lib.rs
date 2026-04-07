//! Provides physics world management, colliders, and raycasting functionalities using the Rapier physics engine.
//!
//! # Features
//! - **Colliders**: Add physics colliders to entities for collision detection (see [`Collider`](crate::colliders::Collider) for supported shapes)
//! - **Raycasting**: Cast rays through the physics world to detect intersections (see [`PhysicsWorld`](crate::prelude::PhysicsWorld) for raycasting methods)
//! - **Automatic Updates**: Colliders automatically sync with entity transforms
//!
//! # Creating Colliders
//! Attach a collider component to any entity with a transform:
//! ```rust
//! commands.spawn((
//!     Transform::from_xyz(0.0, -1.0, 0.0),
//!     Collider::cuboid(50.0, 0.1, 50.0),
//! ));
//! ```
//!
//! # Raycasting
//! Cast rays to detect physics intersections:
//! ```rust
//! fn cast_ray_system(phworld: Res<PhysicsWorld>) {
//!     let ray = Ray::new(
//!         Vec3::new(0.0, 10.0, 0.0), // Origin above the ground
//!         Vec3::new(0.0, -1.0, 0.0), // Direction pointing downwards
//!     );
//!
//!     // Cast the ray and check for hits
//!     if let Some((entity, toi)) = phworld.cast_ray(&ray, &RayCastConfig::default()) {
//!         let hit_point = ray.point_at(toi);
//!         println!("Hit entity {:?} at {:?}", entity, hit_point);
//!     }
//! }
//! ```
//!
//! # Camera-Based Raycasting
//! Cast rays from screen coordinates (useful for mouse picking):
//! ```rust
//! fn mouse_picking(
//!     mut commands: Commands,
//!     phworld: Res<PhysicsWorld>,
//!     window: Single<&Window, With<PrimaryWindow>>,
//!     camera_query: Query<(&Transform, &CameraView), With<Camera>>,
//! ) {
//!     // Get cursor position in NDC
//!     let Some(cursor_pos) = window.cursor_position() else { return };
//!     let cursor_ndc = cursor_pos / window.size();
//!
//!     // Get camera data
//!     let Ok((camera_transform, camera_view)) = camera_query.get_single() else { return };
//!     let aspect_ratio = window.size().x / window.size().y;
//!
//!     // Create ray from camera
//!     let ray = Ray::from_ndc(cursor_ndc, aspect_ratio, camera_transform, camera_view);
//!
//!     // Cast the ray
//!     if let Some((entity, toi)) = phworld.cast_ray(&ray, &RayCastConfig::default()) {
//!         let hit_point = ray.point_at(toi);
//!         println!("Clicked on entity {:?} at {:?}", entity, hit_point);
//!     }
//! }
//! ```
use bevy::prelude::*;

use crate::core::{PhysicsWorld, handle_changes};

#[doc(hidden)]
pub mod prelude {
    pub use crate::colliders::Collider;
    pub use crate::core::PhysicsWorld;
    pub use crate::raycasting::{Ray, RayCastConfig};
}

mod colliders;
mod core;
mod raycasting;

pub struct PhysicsPlugin;
impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PhysicsWorld>()
            .add_systems(Update, handle_changes);
    }
}
