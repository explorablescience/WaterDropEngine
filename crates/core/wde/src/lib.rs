//! WaterDropEngine - A modular game engine.
//!
//! # Overview
//! WaterDropEngine is a modular game engine built on top of Bevy's ECS. It provides a collection of plugins and tools.
//! The engine is composed of several modules, organized from low-level to high-level, each behind
//! a feature of the same name (see [Features](#features)):
//! - Terrain:
//!    - [`wde_terrain`](wde_terrain): A plugin for loading, rendering and colliding chunked heightmap terrains.
//!    - [`wde_terrain_editor`](wde_terrain_editor): A plugin for painting terrains in-game.
//! - Render:
//!    - [`wde_pbr`](wde_pbr): A plugin for physically based rendering (PBR) materials and lighting.
//!    - [`wde_pbr_outline`](wde_pbr_outline): A plugin for drawing outlines around PBR meshes.
//!    - [`wde_camera`](wde_camera): A plugin for managing cameras and viewports.
//!    - [`wde_camera_controller`](wde_camera_controller): A plugin for controlling cameras with various input methods.
//!    - [`wde_gltf`](wde_gltf): A plugin for loading and rendering glTF 3D models.
//!    - [`wde_animation`](wde_animation): A plugin for animating rigid parts of models (e.g. walk cycles).
//!    - [`wde_gizmos`](wde_gizmos): A plugin for rendering debug gizmos and visualizations.
//!    - [`wde_ui`](wde_ui): A plugin for laying out and drawing game UI with taffy.
//! - Core (always included, except the editor):
//!    - [`wde_renderer`](wde_renderer): A plugin for rendering 3D graphics using the `wgpu` graphics API.
//!    - [`wde_scene`](wde_scene): A plugin for managing scenes, entities, and components.
//!    - [`wde_editor`](wde_editor): A plugin for creating in-game editors and tools.
//! - Wrappers:
//!    - `wde_wgpu`: A wrapper around the `wgpu` graphics API for rendering. It is superseeded by [`wde_renderer`](wde_renderer).
//!    - [`wde_physics`](wde_physics): A wrapper around the `rapier` physics engine for 3D physics simulation.
//!    - [`wde_logger`](wde_logger): A wrapper around the `tracing` library for logging and diagnostics.
//!    - `wde_egui`: A wrapper around the `egui` library for creating user interfaces. It is superseeded by [`wde_editor`](wde_editor).
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
//! Each module is enabled by a Cargo feature, which also enables the modules it depends on. Three
//! presets are provided:
//! - `minimal` (default): PBR rendering, cameras with their controllers, and glTF models.
//! - `full`: every runtime module (`minimal`, `atmosphere`, `pbr-outline`, `gizmos`, `ui`,
//!   `physics`, `terrain`).
//! - `everything`: `full`, plus the development tools (`editor`, `terrain-editor`).
//!
//! Without default features, only the renderer, the logger and the scenes are included:
//! ```toml
//! [dependencies]
//! wde = { version = "0.1", default-features = false, features = ["pbr", "ui"] }
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

/// Custom WaterDropEngine plugins. Set it in [`WdeDefaultPlugins`] to configure the window:
/// ```rust
/// app.add_plugins(WdeDefaultPlugins.set(CustomWdePlugins {
///     window_title: "My Game".into(),
///     ..Default::default()
/// }));
/// ```
pub struct CustomWdePlugins {
    /// Title shown in the window's title bar.
    pub window_title: String,
    /// Initial size (width, height) of the window, in logical pixels.
    pub window_resolution: (u32, u32),
    /// Icon shown in the OS taskbar/title bar. See
    /// [`WindowIcon::from_bytes`](wde_renderer::prelude::WindowIcon::from_bytes).
    pub window_icon: Option<wde_renderer::prelude::WindowIcon>
}
impl Default for CustomWdePlugins {
    fn default() -> Self {
        let render = wde_renderer::RenderPlugin::default();
        Self {
            window_title: render.window_title,
            window_resolution: render.window_resolution,
            window_icon: render.window_icon
        }
    }
}
impl Plugin for CustomWdePlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            wde_logger::LogPlugin::default().auto_level(),
            wde_renderer::RenderPlugin {
                window_title: self.window_title.clone(),
                window_resolution: self.window_resolution,
                window_icon: self.window_icon.clone()
            }
        ));

        #[cfg(feature = "gltf")]
        app.add_plugins(wde_gltf::GltfPlugin);
        #[cfg(feature = "animation")]
        app.add_plugins(wde_animation::AnimationPlugin);
        #[cfg(feature = "camera")]
        app.add_plugins(wde_camera::CameraPlugin);
        #[cfg(feature = "camera-controller")]
        app.add_plugins(wde_camera_controller::CameraControllerPlugin);
        #[cfg(feature = "physics")]
        app.add_plugins(wde_physics::PhysicsPlugin);
        #[cfg(feature = "terrain")]
        app.add_plugins(wde_terrain::TerrainPlugin);
        #[cfg(feature = "terrain-editor")]
        app.add_plugins(wde_terrain_editor::TerrainEditorPlugin);
        #[cfg(feature = "pbr-outline")]
        app.add_plugins(wde_pbr_outline::PbrOutlinePlugin);
        #[cfg(feature = "gizmos")]
        app.add_plugins(wde_gizmos::GizmosPlugin);
        #[cfg(feature = "ui")]
        app.add_plugins(wde_ui::UiPlugin);
        #[cfg(feature = "pbr")]
        app.add_plugins(wde_pbr::PbrPlugin);
        #[cfg(feature = "editor")]
        app.add_plugins(wde_editor::EditorPlugin);
        #[cfg(all(feature = "gizmos", feature = "editor"))]
        app.add_plugins(debug::CameraDebugPlugin);
        #[cfg(all(feature = "gizmos", feature = "editor", feature = "physics"))]
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
    pub use wde_logger::prelude::*;
    pub use wde_renderer::prelude::*;
    pub use wde_scene::prelude::*;

    // Optional modules
    #[cfg(all(feature = "gizmos", feature = "editor"))]
    pub use crate::debug::CameraDebugSettings;
    #[cfg(all(feature = "gizmos", feature = "editor", feature = "physics"))]
    pub use crate::debug::PhysicsDebugSettings;
    #[cfg(feature = "animation")]
    pub use wde_animation::prelude::*;
    #[cfg(feature = "camera")]
    pub use wde_camera::prelude::*;
    #[cfg(feature = "camera-controller")]
    pub use wde_camera_controller::prelude::*;
    #[cfg(feature = "editor")]
    pub use wde_editor::prelude::*;
    #[cfg(feature = "gizmos")]
    pub use wde_gizmos::prelude::*;
    #[cfg(feature = "gltf")]
    pub use wde_gltf::prelude::*;
    #[cfg(feature = "pbr")]
    pub use wde_pbr::prelude::*;
    #[cfg(feature = "pbr-outline")]
    pub use wde_pbr_outline::prelude::*;
    #[cfg(feature = "physics")]
    pub use wde_physics::prelude::*;
    #[cfg(feature = "terrain")]
    pub use wde_terrain::prelude::*;
    #[cfg(feature = "ui")]
    pub use wde_ui::prelude::*;
}

// Re-export the modules for external use
pub use wde_logger;
pub use wde_renderer;
pub use wde_scene;

#[cfg(feature = "animation")]
pub use wde_animation;
#[cfg(feature = "camera")]
pub use wde_camera;
#[cfg(feature = "camera-controller")]
pub use wde_camera_controller;
#[cfg(feature = "editor")]
pub use wde_editor;
#[cfg(feature = "gizmos")]
pub use wde_gizmos;
#[cfg(feature = "gltf")]
pub use wde_gltf;
#[cfg(feature = "pbr")]
pub use wde_pbr;
#[cfg(feature = "pbr-outline")]
pub use wde_pbr_outline;
#[cfg(feature = "physics")]
pub use wde_physics;
#[cfg(feature = "terrain")]
pub use wde_terrain;
#[cfg(feature = "terrain-editor")]
pub use wde_terrain_editor;
#[cfg(feature = "ui")]
pub use wde_ui;
