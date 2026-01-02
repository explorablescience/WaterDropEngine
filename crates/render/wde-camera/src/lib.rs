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

