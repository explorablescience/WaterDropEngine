use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::view::{CameraUniform, CameraView};

pub(crate) struct CameraFeature;
impl Plugin for CameraFeature {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderBindingPluginRegister::<CameraRender>::default());
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<CameraUniform>()
            .add_systems(Extract, extract)
            .add_systems(Render, update_buffer);

        // Register types for reflection
        app.register_type::<Camera>().register_type::<CameraView>();
    }
}

/// A camera is defined by a position and a view.
#[derive(Component, Default, Clone, Debug, Reflect)]
#[reflect(Component)]
#[require(Transform, CameraView)]
pub struct Camera;

/// Marker component for the active camera. The render feature reads this each frame to know which camera to use for rendering.
#[derive(Component, Default, Clone, Debug)]
#[require(Transform, CameraView, Camera)]
pub struct ActiveCamera;

/// Camera bind group that shaders can consume.
/// This is the data that will be sent to the GPU each frame for the active camera. It contains the world-to-view and view-to-ndc matrices, as well as the camera position.
#[derive(Asset, Clone, TypePath, Default)]
pub struct CameraRender;
impl RenderBinding for CameraRender {
    fn describe(&self, builder: &mut RenderBindingBuilder) {
        builder.add_buffer(
            0,
            Buffer {
                label: "camera".to_string(),
                size: std::mem::size_of::<CameraUniform>(),
                usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                content: None
            }
        );
    }

    fn label(&self) -> &'static str {
        "camera"
    }
}

fn extract(
    cameras: ExtractWorld<Query<(&Transform, &CameraView), With<ActiveCamera>>>,
    mut camera_uniform: ResMut<CameraUniform>,
    window: ExtractWorld<Query<&Window>>
) {
    if let (Ok((transform, view)), Ok(window)) = (cameras.single(), window.single()) {
        // Update the camera uniform
        let aspect_ratio = window.width() / window.height();
        *camera_uniform = CameraUniform::new(transform, view, aspect_ratio);
    }
}

fn update_buffer(
    render_instance: Res<RenderInstance>,
    camera_uniform: Res<CameraUniform>,
    camera_buffer: Res<RenderAssets<GpuRenderBinding<CameraRender>>>,
    mut buffers: ResMut<RenderAssets<GpuBuffer>>
) {
    let camera_binding = match camera_buffer.iter().next() {
        Some((_, binding)) => binding,
        None => return
    };
    if let Some(camera_buffer) = buffers.get_mut(camera_binding.get_buffer(0).unwrap()) {
        let render_instance = render_instance.0.read().unwrap();
        camera_buffer.buffer.write(
            &render_instance,
            bytemuck::cast_slice(&[camera_uniform.to_owned()]),
            0
        );
    }
}
