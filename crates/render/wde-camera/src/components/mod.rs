use bevy::prelude::*;

mod camera;
mod free_camera_controller;
mod third_person_controller;

pub use camera::*;
pub use free_camera_controller::*;
pub use third_person_controller::*;

pub(crate) struct RenderComponentsPlugin;
impl Plugin for RenderComponentsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(FreeCameraControllerPlugin)
            .add_plugins(ThirdPersonControllerPlugin);

        // Register the components to the reflect system
        app
            .register_type::<FreeCameraController>()
            .register_type::<ActiveCamera>()
            .register_type::<CameraView>()
            .register_type::<Camera>();
    }
}

