//! WaterDropEngine's `wde-camera` crate provides the camera ECS pieces that feed render-time
//! matrices into `wde-renderer`. It bundles first-person controls, projection defaults, an
//! active-camera tag, and the render-side bind group/buffer that shaders consume.
//!
//! # Architecture
//! - **Components**: [`components::CameraView`] stores fov/near/far; [`components::Camera`]
//!   tags an entity as a camera; [`components::ActiveCamera`] marks the one to render; and
//!   [`components::CameraController`] implements a FPS-style mover.
//! - **Controller system**: The controller system polls keyboard/mouse,
//!   updates yaw/pitch, velocity, and writes back the entity `Transform`.
//! - **Render feature**: [`features::CameraFeatureRender`] allocates a uniform buffer +
//!   bind group (layout at binding 0) with world→NDC, NDC→world, and camera position.
//! - **Plugin wiring**: [`CameraPlugin`] registers the components for reflection, installs the
//!   controller system, extracts the active camera every frame, and keeps the GPU buffer fresh.
//!
//! # Quickstart (FPS camera)
//! ```rust,no_run
//! use bevy::prelude::*;
//! use wde_renderer::prelude::*;
//! use wde_camera::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(RenderPlugin)   // from wde-renderer
//!         .add_plugins(CameraPlugin)
//!         .add_systems(Startup, spawn_camera)
//!         .run();
//! }
//!
//! fn spawn_camera(mut commands: Commands) {
//!     commands.spawn((
//!         Camera,
//!         CameraView::default(),   // 60° fov, 0.1..1000 z range
//!         CameraController::default(), // WASD + mouse look
//!         ActiveCamera,
//!         Transform::from_xyz(0.0, 2.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
//!     ));
//! }
//! ```
//!
//! # Core usage patterns
//! - Keep exactly one `ActiveCamera` in the main world; the render feature reads it each frame.
//! - Tune `CameraController` keys/speeds/sensitivity per scene; yaw/pitch are persisted.
//! - When resizing the window, the feature recomputes aspect ratio automatically from `Window`.
//! - The camera bind group sits at index 0 in render pipelines; reuse it across passes.
//!
//! # Modules
//! - [`components`]: camera tags, projection data, controller system, GPU uniform packing.
//! - [`features`]: render-side extraction, bind group layout, and buffer upload schedule.
//!
//! # Examples and further reading
//! - For custom movement, swap out `CameraController` but keep `Camera`, `CameraView`, and
//!   `ActiveCamera` so the uniform still updates.
//! - Use the camera uniforms directly in WGSL (`world_to_ndc`, `ndc_to_world`, `position`)
//!   when writing pipelines in `wde-renderer`.

#![allow(clippy::type_complexity)]
use crate::{components::RenderComponentsPlugin, features::RenderFeaturesPlugin};
use bevy::prelude::*;

pub mod prelude {
    pub use crate::CameraPlugin;
    pub use crate::components::{CameraController, CameraView, ActiveCamera, Camera};
    pub use crate::features::CameraFeatureRender;
}

pub mod components;
pub mod features;

pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(RenderComponentsPlugin)
            .add_plugins(RenderFeaturesPlugin);
    }
}

