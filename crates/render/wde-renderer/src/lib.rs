#![allow(clippy::just_underscores_and_digits)]
#![allow(clippy::type_complexity)]

pub mod assets;
pub mod pipelines;
pub mod components;
pub mod core;
pub mod features;
pub mod passes;
pub mod utils;

use core::RenderCorePlugin;

use assets::AssetsPlugin;
use bevy::{app::{App, Plugin}, log::info};

pub struct RenderPlugin;
impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        // First, add the renderer plugin
        app.add_plugins(RenderCorePlugin);

        // Register the scene plugin
        app.add_plugins(AssetsPlugin);
    }

    fn finish(&self, _app: &mut App) {
        info!("Render plugin initialized.");
    }
}
