use bevy::prelude::*;

#[cfg(debug_assertions)]
use crate::editor::CameraPropertiesEditor;
use crate::{
    free_camera_controller::FreeCameraControllerPlugin,
    third_person_controller::ThirdPersonControllerPlugin
};

pub mod prelude {
    pub use crate::free_camera_controller::FreeCameraController;
    pub use crate::third_person_controller::ThirdPersonController;
}

mod editor;
mod free_camera_controller;
mod third_person_controller;

pub struct CameraControllerPlugin;
impl Plugin for CameraControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FreeCameraControllerPlugin)
            .add_plugins(ThirdPersonControllerPlugin);

        #[cfg(debug_assertions)]
        app.add_plugins(CameraPropertiesEditor);
    }
}
