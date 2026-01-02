#![allow(clippy::just_underscores_and_digits)]
#![allow(clippy::type_complexity)]

use wde::prelude::*;
use bevy::input::InputPlugin;
use bevy::app::TaskPoolThreadAssignmentPolicy;
use bevy::prelude::*;
use bevy::log::{Level, LogPlugin};
use wde::scene::ScenePlugin;

pub fn main() {
    // Log level
    #[cfg(debug_assertions)]
    let level = if cfg!(feature = "trace") {
        Level::TRACE
    } else {
        Level::DEBUG
    };
    #[cfg(not(debug_assertions))]
    let level = Level::INFO;

    // Create the app
    let mut app = App::new();

    // Add default bevy plugins
    app
        .add_plugins(MinimalPlugins.set(TaskPoolPlugin {
            task_pool_options: TaskPoolOptions {
                min_total_threads: 1,
                max_total_threads: usize::MAX,

                // Use 1 core for IO
                io: TaskPoolThreadAssignmentPolicy {
                    min_threads: 1,
                    max_threads: 1,
                    percent: 0.25,
                    on_thread_spawn: None,
                    on_thread_destroy: None,
                },

                // Use 1 core for async compute
                async_compute: TaskPoolThreadAssignmentPolicy {
                    min_threads: 1,
                    max_threads: 1,
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
        }))
        .add_plugins(LogPlugin {
            level,
            filter: "wgpu_hal=warn,wgpu_core=warn,naga=warn".to_string(),
            custom_layer: |_| None,
            fmt_layer: |_| None,
        })
        // .add_plugins(HierarchyPlugin)
        .add_plugins(InputPlugin)
        .add_plugins(AssetPlugin {
            mode: AssetMode::Unprocessed,
            file_path: "res".to_string(),
            ..Default::default()
        });
    info!("Starting game engine.");

    // Add the plugins
    app
        .add_plugins(RenderPlugin)
        .add_plugins(ScenePlugin)
        .add_plugins(GizmosPlugin)
        .add_plugins(PbrPlugin);
        // .add_plugins(GamePlugin);

    // Run the app
    info!("Running game engine.");
    app.run();
}
