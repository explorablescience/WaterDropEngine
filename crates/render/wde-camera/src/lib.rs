//! Camera plugin for WaterDropEngine.
//!
//! This plugin provides a camera component and a camera view component, as well as a system to extract the active camera and update the camera uniform each frame.
//!
//! # Example
//! To spawn a camera, add a [`crate::prelude::Camera`] component, a [`crate::prelude::CameraView`] component. If you want the camera to be active, add the [`crate::prelude::ActiveCamera`] component as well. For example:
//! ```rust
//! fn spawn_camera(mut commands: Commands) {
//!     commands.spawn((
//!         Transform::from_xyz(0.0, 2.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
//!         Camera,
//!         CameraView::default(),
//!         ActiveCamera
//!     ));
//! }
//! ```
//!
//! The camera uniform will then be automatically updated each frame with the active camera's transform and view parameters.
//! To consumes the camera uniform in a shader, you can use the [`crate::camera::CameraRender`] render binding.
//! It is accessible by the resource `Res<RenderAssets<GpuRenderBinding<CameraRender>>>` in render systems, or `SBindings<CameraRender>` in shader code.
//! See [`wde_renderer::prelude::GpuRenderBinding`] for more details on how to use render bindings in shaders.
use bevy::prelude::*;

use crate::camera::CameraFeature;

#[doc(hidden)]
pub mod prelude {
    pub use crate::camera::{ActiveCamera, Camera, CameraRender};
    pub use crate::view::CameraView;
}

mod camera;
mod view;

#[cfg(debug_assertions)]
mod editor;

pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CameraFeature);

        #[cfg(debug_assertions)]
        app.add_plugins(editor::CameraPropertiesEditor);
    }
}
