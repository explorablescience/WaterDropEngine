use bevy::app::{App, Plugin};

mod camera;
mod lights;

pub use camera::*;
pub use lights::*;

pub struct RenderFeaturesPlugin;
impl Plugin for RenderFeaturesPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(CameraFeature)
            .add_plugins(LightsFeature);
    }
}
