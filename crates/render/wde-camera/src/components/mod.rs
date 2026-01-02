use bevy::prelude::*;

mod camera;
mod camera_controller;

pub use camera::*;
pub use camera_controller::*;

pub(crate) struct RenderComponentsPlugin;
impl Plugin for RenderComponentsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(CameraControllerPlugin);

        // Register the components to the reflect system
        app
            .register_type::<CameraController>()
            .register_type::<ActiveCamera>()
            .register_type::<CameraView>()
            .register_type::<Camera>();
    }
}

