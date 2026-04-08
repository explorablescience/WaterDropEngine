//! A collection of camera controllers for Bevy.
//!
//! This crate provides a set of camera controllers that can be easily integrated the applications.
//! It includes a:
//!  - [`FreeCameraController`](free_camera_controller::FreeCameraController) for first-person navigation.
//!  - [`ThirdPersonController`](third_person_controller::ThirdPersonController) for third-person perspectives.
//!
//! To use the camera controllers, simply add the corresponding controller component to your camera entity.
//! For example, to use the [`ThirdPersonController`](third_person_controller::ThirdPersonController), you can spawn a camera entity like this:
//!
//! ```rust
//! commands.spawn((
//!    Transform::from_xyz(0.0, 2.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
//!    ActiveCamera,
//!    ThirdPersonController::default(),
//! ));
//!```

use bevy::prelude::*;

#[cfg(debug_assertions)]
use crate::editor::CameraPropertiesEditor;
use crate::{
    free_camera_controller::FreeCameraControllerPlugin,
    third_person_controller::ThirdPersonControllerPlugin
};

#[doc(hidden)]
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
        app.add_plugins((FreeCameraControllerPlugin, ThirdPersonControllerPlugin));

        #[cfg(debug_assertions)]
        app.add_plugins(CameraPropertiesEditor);
    }
}
