//! WaterDropEngine - A modular game engine built on top of Bevy.
//!
//! WaterDropEngine (WDE) is a collection of rendering, physics, and scene management systems
//! designed to work seamlessly with the Bevy game engine. It provides high-level abstractions
//! for common game development tasks while maintaining flexibility and performance.
//!
//! # Features
//!
//! - **Rendering**: Advanced rendering pipeline with support for custom materials and passes
//! - **Camera System**: Flexible camera management with projection and view transformations
//! - **Physics**: Rapier-based physics simulation with colliders and raycasting
//! - **PBR Rendering**: Physically-based rendering materials and lighting (optional, feature-gated)
//! - **Gizmos**: Debug visualization tools (optional, feature-gated)
//! - **Scene Management**: Entity and component management for game scenes
//!
//! # Quick Start
//!
//! Add WaterDropEngine to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! wde = { path = "path/to/wde" }
//! ```
//!
//! For a basic setup with all features:
//!
//! ```toml
//! [dependencies]
//! wde = { path = "path/to/wde", features = ["gizmos", "pbr"] }
//! ```
//!
//! # Basic Example
//!
//! ```no_run
//! use bevy::prelude::*;
//! use wde::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(WdeRendererPlugin)
//!         .add_plugins(CameraPlugin)
//!         .add_plugins(PhysicsPlugin)
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands) {
//!     // Spawn a camera
//!     commands.spawn((
//!         Camera,
//!         Transform::from_xyz(0.0, 5.0, 10.0)
//!             .looking_at(Vec3::ZERO, Vec3::Y),
//!     ));
//!
//!     // Spawn an object with a collider
//!     commands.spawn((
//!         Transform::from_xyz(0.0, 0.0, 0.0),
//!         Collider::cuboid(1.0, 1.0, 1.0),
//!     ));
//! }
//! ```
//!
//! # Module Organization
//!
//! WDE is organized into several submodules:
//!
//! - [`render`]: Core rendering functionality and custom render passes
//! - [`camera`]: Camera components and systems
//! - [`scene`]: Scene management and entity organization
//! - [`gizmos`]: Debug visualization and gizmo rendering (requires `gizmos` feature)
//! - [`pbr`]: Physically-based rendering materials (requires `pbr` feature)
//!
//! # Prelude
//!
//! The [`prelude`] module re-exports the most commonly used types and traits.
//! Import it to get started quickly:
//!
//! ```
//! use wde::prelude::*;
//! ```
//!
//! # Feature Flags
//!
//! - `gizmos`: Enables debug visualization and gizmo rendering tools
//! - `pbr`: Enables physically-based rendering materials and lighting systems
//!
//! # Physics Integration
//!
//! WDE includes a physics module powered by Rapier3D:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use wde::prelude::*;
//! fn spawn_physics_objects(mut commands: Commands) {
//!     // Create a ground plane
//!     commands.spawn((
//!         Transform::from_xyz(0.0, -1.0, 0.0),
//!         Collider::cuboid(50.0, 0.1, 50.0),
//!     ));
//!
//!     // Cast a ray
//!     # /*
//!     let ray = Ray::new(Vec3::Y * 10.0, Vec3::NEG_Y);
//!     if let Some((entity, toi)) = physics_world.cast_ray(&ray, &RayCastConfig::default()) {
//!         println!("Hit entity {:?} at distance {}", entity, toi);
//!     }
//!     # */
//! }
//! ```

use bevy::{app::{ScheduleRunnerPlugin, TaskPoolThreadAssignmentPolicy, plugin_group}, diagnostic::FrameCountPlugin, input::InputPlugin, prelude::*, time::TimePlugin};

/// Custom Bevy plugins for WaterDropEngine default plugins.
#[derive(Default)]
struct CustomBevyPlugins;
impl Plugin for CustomBevyPlugins {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(TaskPoolPlugin {
                task_pool_options: TaskPoolOptions {
                    min_total_threads: 1,
                    max_total_threads: usize::MAX,

                    // Use 1 core for IO
                    io: TaskPoolThreadAssignmentPolicy {
                        min_threads: 1,
                        max_threads: 2,
                        percent: 0.25,
                        on_thread_spawn: None,
                        on_thread_destroy: None,
                    },

                    // Use 1 core for async compute
                    async_compute: TaskPoolThreadAssignmentPolicy {
                        min_threads: 1,
                        max_threads: 2,
                        percent: 0.25,
                        on_thread_spawn: None,
                        on_thread_destroy: None,
                    },

                    // Use all remaining cores for compute (at least 1)
                    compute: TaskPoolThreadAssignmentPolicy {
                        min_threads: 1,
                        max_threads: usize::MAX,
                        percent: 1.0, // This 1.0 here means "whatever is left over"
                        on_thread_spawn: None,
                        on_thread_destroy: None,
                    },
                }
            })
            .add_plugins(FrameCountPlugin)
            .add_plugins(TimePlugin)
            .add_plugins(ScheduleRunnerPlugin::default())
            .add_plugins(bevy::prelude::AssetPlugin {
                mode: AssetMode::Unprocessed,
                file_path: "res".to_string(),
                ..Default::default()
            })
            .add_plugins(InputPlugin);
    }
}

/// Custom WaterDropEngine plugins.
#[derive(Default)]
struct CustomWdePlugins;
impl Plugin for CustomWdePlugins {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(wde_logger::LogPlugin::default().auto_level())
            .add_plugins(wde_renderer::RenderPlugin)
            .add_plugins(wde_camera::CameraPlugin)
            .add_plugins(wde_physics::PhysicsPlugin);

        #[cfg(feature = "gizmos")]
        app.add_plugins(wde_gizmos::GizmosPlugin);
        #[cfg(feature = "pbr")]
        app.add_plugins(wde_pbr::PbrPlugin);
        #[cfg(feature = "egui")]
        app.add_plugins(wde_egui::EguiPlugin);

        app.add_plugins(wde_scene::ScenePlugin);
    }
}

plugin_group! {
    /// Default plugins for WaterDropEngine.
    /// 
    /// # Description
    /// 
    /// To use, add the following to your app;
    /// ```rust
    /// app.add_plugins(WdeDefaultPlugins);
    /// ```
    /// 
    /// # Modifying Plugins
    /// 
    /// To modify and configure individual plugins, use:
    /// ```rust
    /// app.add_plugins(WdeDefaultPlugins.set(LogPlugin {
    ///     level: Level::DEBUG,
    ///     ..Default::default()
    /// }));
    /// ```
    pub struct WdeDefaultPlugins {
        :CustomBevyPlugins,
        :CustomWdePlugins,
    }
}

/// The prelude module.
///
/// Import this module to get access to the most commonly used types and traits:
///
/// ```
/// use wde::prelude::*;
/// ```
///
/// This includes:
/// - Renderer core types and plugins
/// - Camera components and systems
/// - Physics types (colliders, raycasting)
/// - Gizmo types (if `gizmos` feature is enabled)
/// - PBR materials (if `pbr` feature is enabled)
pub mod prelude {
    // Default plugins
    pub use crate::WdeDefaultPlugins;

    // Core modules
    pub use wde_logger::prelude::*;
    pub use wde_renderer::prelude::*;
    pub use wde_camera::prelude::*;
    pub use wde_physics::prelude::*;

    // Optional feature modules
    #[cfg(feature = "gizmos")]
    pub use wde_gizmos::prelude::*;

    #[cfg(feature = "pbr")]
    pub use wde_pbr::prelude::*;

    #[cfg(feature = "egui")]
    pub use wde_egui::prelude::*;
}

/// Rendering module.
/// 
/// Provides core rendering functionality:
/// - Custom render passes
/// - Material management
/// - Shader handling
pub mod render {
    pub use wde_renderer::*;
}

/// Camera module.
///
/// Provides camera components and utilities:
/// - Camera view and projection settings
/// - Screen-space to world-space transformations
/// - NDC coordinate conversions
pub mod camera {
    pub use wde_camera::*;
}

/// Scene management module.
///
/// Handles scene organization and entity management:
/// - Scene loading and saving
/// - Entity hierarchies
/// - Component serialization
pub mod scene {
    pub use wde_scene::*;
}

/// Gizmo rendering module (requires `gizmos` feature).
///
/// Provides debug visualization tools:
/// - Lines and shapes
/// - Grid rendering
/// - Debug overlays
/// - Visual debugging aids
#[cfg(feature = "gizmos")]
pub mod gizmos {
    pub use wde_gizmos::*;
}

/// Physically-based rendering module (requires `pbr` feature).
///
/// Provides PBR materials and lighting:
/// - Metallic-roughness workflow
/// - Image-based lighting
/// - Normal mapping
/// - Advanced material properties
#[cfg(feature = "pbr")]
pub mod pbr {
    pub use wde_pbr::*;
}

