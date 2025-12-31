use bevy::prelude::*;

mod transform;
mod camera;
mod camera_controller;
mod lights;

pub use transform::*;
pub use camera::*;
pub use camera_controller::*;
pub use lights::*;

pub struct RenderComponentsPlugin;
impl Plugin for RenderComponentsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(CameraControllerPlugin);

        // Register the components to the reflect system
        app
            .register_type::<CameraController>()
            .register_type::<ActiveCamera>()
            .register_type::<CameraView>()
            .register_type::<Camera>()
            .register_type::<DirectionalLight>()
            .register_type::<PointLight>()
            .register_type::<SpotLight>();
    }
}

