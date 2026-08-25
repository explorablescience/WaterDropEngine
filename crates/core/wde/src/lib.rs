//! WaterDropEngine - A modular game engine.
//!
//! # Overview
//! WaterDropEngine is a modular game engine built on top of Bevy's ECS. It provides a collection of plugins and tools.
//! The engine is composed of several modules, organized from low-level to high-level:
//! - High-Level:
//!    - [`wde_terrain`](wde_terrain): A plugin for generating and rendering procedural terrain.
//!    - [`wde_terrain_editor`](wde_terrain_editor): A plugin for editing terrain in-game.
//!    - [`wde_terrain_grid`](wde_terrain_grid): A plugin for managing terrain grids and LOD.
//!    - [`wde_terrain_navigation`](wde_terrain_navigation): A plugin for pathfinding and navigation on terrain.
//! - Render:
//!    - [`wde_pbr`](wde_pbr): A plugin for physically based rendering (PBR) materials and lighting.
//!    - [`wde_camera`](wde_camera): A plugin for managing cameras and viewports.
//!    - [`wde_camera_controller`](wde_camera_controller): A plugin for controlling cameras with various input methods.
//!    - [`wde_gltf`](wde_gltf): A plugin for loading and rendering glTF 3D models.
//!    - [`wde_gizmos`](wde_gizmos): A plugin for rendering debug gizmos and visualizations.
//! - Core:
//!    - [`wde_renderer`](wde_renderer): A plugin for rendering 3D graphics using the `wgpu` graphics API.
//!    - [`wde_scene`](wde_scene): A plugin for managing scenes, entities, and components.
//!    - [`wde_editor`](wde_editor): A plugin for creating in-game editors and tools.
//! - Wrappers:
//!    - [`wde_wgpu`](wde_wgpu): A wrapper around the `wgpu` graphics API for rendering. It is superseeded by [`wde_renderer`](wde_renderer).
//!    - [`wde_physics`](wde_physics): A wrapper around the `rapier` physics engine for 3D physics simulation.
//!    - [`wde_logger`](wde_logger): A wrapper around the `tracing` library for logging and diagnostics.
//!    - [`wde_egui`](wde_egui): A wrapper around the `egui` library for creating user interfaces. It is superseeded by [`wde_editor`](wde_editor).
//!
//! # Getting Started
//! To get started with WaterDropEngine, add the `wde` crate to your `Cargo.toml`:
//! ```toml
//! [dependencies]
//! wde = "0.1"
//! ```
//! Then, in your main Rust file, you can use the default plugins to set up a basic application:
//! ```rust
//! app.add_plugins(WdeDefaultPlugins);
//! ```
//!
//! To modify and configure individual plugins, use:
//! ```rust
//! app.add_plugins(WdeDefaultPlugins.set(LogPlugin {
//!     level: LogLevel::DEBUG,
//!     ..Default::default()
//! }));
//! ```
//!
//! # Features
//! WaterDropEngine is designed to be modular and extensible. You can enable or disable features! using Cargo features. For example, to enable the `pbr` feature for physically based rendering, add the following to your `Cargo.toml`:
//! ```toml
//! [dependencies]
//! wde = { version = "0.1", features = ["pbr"] }
//! ```

use bevy::{
    app::{ScheduleRunnerPlugin, TaskPoolThreadAssignmentPolicy, plugin_group},
    diagnostic::FrameCountPlugin,
    input::InputPlugin,
    prelude::*,
    time::TimePlugin
};

#[cfg(all(feature = "gizmos", feature = "editor"))]
mod debug;

/// Custom Bevy plugins for WaterDropEngine default plugins.
#[derive(Default)]
pub struct CustomBevyPlugins;
impl Plugin for CustomBevyPlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            TaskPoolPlugin {
                task_pool_options: TaskPoolOptions {
                    min_total_threads: 1,
                    max_total_threads: usize::MAX,

                    // Use 1 core for IO
                    io: TaskPoolThreadAssignmentPolicy {
                        min_threads: 1,
                        max_threads: 2,
                        percent: 0.25,
                        on_thread_spawn: None,
                        on_thread_destroy: None
                    },

                    // Use 1 core for async compute
                    async_compute: TaskPoolThreadAssignmentPolicy {
                        min_threads: 1,
                        max_threads: 2,
                        percent: 0.25,
                        on_thread_spawn: None,
                        on_thread_destroy: None
                    },

                    // Use all remaining cores for compute (at least 1)
                    compute: TaskPoolThreadAssignmentPolicy {
                        min_threads: 1,
                        max_threads: usize::MAX,
                        percent: 1.0, // This 1.0 here means "whatever is left over"
                        on_thread_spawn: None,
                        on_thread_destroy: None
                    }
                }
            },
            FrameCountPlugin,
            TimePlugin,
            ScheduleRunnerPlugin::default(),
            bevy::prelude::AssetPlugin {
                mode: AssetMode::Unprocessed,
                file_path: "res".to_string(),
                ..Default::default()
            },
            InputPlugin,
            TransformPlugin
        ));
    }
}

/// Custom WaterDropEngine plugins.
#[derive(Default)]
struct CustomWdePlugins;
impl Plugin for CustomWdePlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            wde_logger::LogPlugin::default().auto_level(),
            wde_renderer::RenderPlugin::default(),
            wde_gltf::GltfPlugin,
            wde_camera::CameraPlugin,
            wde_camera_controller::CameraControllerPlugin,
            wde_physics::PhysicsPlugin,
            wde_terrain::TerrainPlugin,
            wde_terrain_grid::TerrainGridPlugin,
            wde_terrain_editor::TerrainEditorPlugin,
            wde_terrain_navigation::TerrainNavigationPlugin,
            wde_pbr_outline::PbrOutlinePlugin
        ));

        #[cfg(feature = "gizmos")]
        app.add_plugins(wde_gizmos::GizmosPlugin);
        #[cfg(feature = "pbr")]
        app.add_plugins(wde_pbr::PbrPlugin);
        #[cfg(feature = "editor")]
        app.add_plugins(wde_editor::EditorPlugin);
        #[cfg(all(feature = "gizmos", feature = "editor"))]
        app.add_plugins(debug::PhysicsDebugPlugin);

        // Always add the scene plugin last
        app.add_plugins(wde_scene::ScenePlugin);
    }
}

plugin_group! {
    /// Default plugins for WaterDropEngine.
    pub struct WdeDefaultPlugins {
        :CustomBevyPlugins,
        :CustomWdePlugins,
    }
}

/// The prelude module of WaterDropEngine.
/// Import this to get access to the most commonly used types and traits.
#[doc(hidden)]
pub mod prelude {
    // Default plugins
    pub use crate::WdeDefaultPlugins;

    // Core modules
    pub use wde_camera::prelude::*;
    pub use wde_camera_controller::prelude::*;
    pub use wde_gltf::prelude::*;
    pub use wde_logger::prelude::*;
    pub use wde_pbr_outline::prelude::*;
    pub use wde_physics::prelude::*;
    pub use wde_renderer::prelude::*;
    pub use wde_scene::prelude::*;
    pub use wde_terrain::prelude::*;
    pub use wde_terrain_grid::prelude::*;
    pub use wde_terrain_navigation::prelude::*;

    // Optional feature modules
    #[cfg(all(feature = "gizmos", feature = "editor"))]
    pub use crate::debug::PhysicsDebugSettings;
    #[cfg(feature = "editor")]
    pub use wde_editor::prelude::*;
    #[cfg(feature = "gizmos")]
    pub use wde_gizmos::prelude::*;
    #[cfg(feature = "pbr")]
    pub use wde_pbr::prelude::*;
}

// Re-export the modules for external use
pub use wde_camera;
pub use wde_camera_controller;
pub use wde_gltf;
pub use wde_logger;
pub use wde_pbr_outline;
pub use wde_physics;
pub use wde_renderer;
pub use wde_scene;
pub use wde_terrain;
pub use wde_terrain_editor;
pub use wde_terrain_grid;
pub use wde_terrain_navigation;

#[cfg(feature = "editor")]
pub use wde_editor;
#[cfg(feature = "gizmos")]
pub use wde_gizmos;
#[cfg(feature = "pbr")]
pub use wde_pbr;
