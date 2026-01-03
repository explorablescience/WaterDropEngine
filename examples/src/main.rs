#![allow(clippy::type_complexity)]
use bevy::input::InputPlugin;
use bevy::prelude::*;
use bevy::log::{Level, LogPlugin};
use wde::prelude::*;
use wde::scene::ScenePlugin;

use crate::custom_forward_render::CustomFeaturesPlugin;
use crate::pbr_batches::PbrBatchesPlugin;

mod custom_forward_render;
mod display_texture;
mod pbr_batches;
mod ray_casting;

#[allow(unused)]
enum Example {
    CustomForwardRender,
    DisplayTexture,
    PbrBatches,
    RayCasting,
}
const SELECTED_EXAMPLE: Example = Example::PbrBatches;

pub fn main() {
    // Log level
    #[cfg(debug_assertions)]
    let level = if cfg!(feature = "tracing") {
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
        .add_plugins(MinimalPlugins)
        .add_plugins(LogPlugin {
            level,
            filter: "wgpu_hal=warn,wgpu_core=warn,naga=warn".to_string(),
            custom_layer: |_| None,
            fmt_layer: |_| None,
        })
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
        .add_plugins(PbrPlugin)
        .add_plugins(GizmosPlugin)
        .add_plugins(CameraPlugin)
        .add_plugins(PhysicsPlugin)
        .add_plugins(ScenePlugin);

    // Start the selected example
    match SELECTED_EXAMPLE {
        Example::CustomForwardRender => {
            info!("Starting Custom Forward Render example.");
            app.add_plugins(CustomFeaturesPlugin);
        }
        Example::DisplayTexture => {
            info!("Starting Display Texture example.");
            app.add_plugins(display_texture::DisplayTextureComponentPlugin);
        }
        Example::PbrBatches => {
            info!("Starting PBR Batches example.");
            app.add_plugins(PbrBatchesPlugin);
        }
        Example::RayCasting => {
            info!("Starting Ray Casting example.");
            app.add_plugins(ray_casting::RayCastingPlugin);
        }
    }

    // Run the app
    info!("Running game engine.");
    app.run();
}
