use bevy::app::{App, Plugin};

use crate::features::camera::CameraFeature;

mod camera;

pub use camera::CameraFeatureRender;

pub(crate) struct RenderFeaturesPlugin;
impl Plugin for RenderFeaturesPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(CameraFeature);
    }
}
