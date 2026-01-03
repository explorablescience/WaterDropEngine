#![allow(clippy::type_complexity)]
use bevy::prelude::*;
use wde::prelude::*;

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
    // Create the app
    let mut app = App::new();

    // Add default plugins
    info!("Adding default plugins.");
    app.add_plugins(WdeDefaultPlugins);

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
