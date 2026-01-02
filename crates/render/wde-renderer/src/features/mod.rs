use bevy::app::{App, Plugin};

mod camera;

pub use camera::*;

pub struct RenderFeaturesPlugin;
impl Plugin for RenderFeaturesPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(CameraFeature);
    }
}
