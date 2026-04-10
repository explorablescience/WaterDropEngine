use bevy::{ecs::system::SystemParamItem, prelude::*};
use wde_renderer::prelude::*;

use crate::view::{CameraUniform, CameraView};

pub(crate) struct CameraFeature;
impl Plugin for CameraFeature {
    fn build(&self, app: &mut App) {
        app.add_plugins((RenderDataRegisterPlugin::<CameraRender>::default(), RenderBindingRegisterPlugin::<CameraBinding>::default()));
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

/// Camera render data buffer. This is used to store the camera uniform data that will be sent to the GPU each frame.
#[derive(Asset, Clone, TypePath, Default)]
pub struct CameraRender;
impl CameraRender {
    pub const CAMERA_IDX: u32 = 0;
}
impl RenderData for CameraRender {
    type Params = ();

    fn describe(_params: &SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        builder.add_buffer(
            Self::CAMERA_IDX,
            Buffer {
                label: "camera".to_string(),
                size: std::mem::size_of::<CameraUniform>(),
                usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                content: None
            }
        );
    }
}

/// Render binding containing only the camera buffer. This is used to bind the camera buffer to the GPU pipeline.
#[derive(Asset, Clone, TypePath, Default)]
pub struct CameraBinding;
impl RenderBinding for CameraBinding {
    type Params = SRenderData<CameraRender>;

    fn describe(&mut self, camera_render: &SystemParamItem<Self::Params>, builder: &mut RenderBindingBuilder) {
        builder.add_buffer(camera_render, CameraRender::CAMERA_IDX);
    }

    fn label(&self) -> &'static str {
        "camera"
    }
}

fn extract(
    cameras: ExtractWorld<Query<(&GlobalTransform, &CameraView), With<ActiveCamera>>>,
    mut camera_uniform: ResMut<CameraUniform>,
    window: ExtractWorld<Query<&Window>>
) {
    // Update the camera uniform
    if let (Ok((transform, view)), Ok(window)) = (cameras.single(), window.single()) {
        let aspect_ratio = window.width() / window.height();
        *camera_uniform = CameraUniform::new(transform, view, aspect_ratio);
    }
}

fn update_buffer(
    render_instance: Res<RenderInstance>,
    camera_uniform: Res<CameraUniform>,
    camera_buffer: ResRenderData<CameraRender>,
    mut buffers: ResMut<RenderAssets<GpuBuffer>>
) {
    let camera_buffer = match camera_buffer.iter().next() {
        Some((_, binding)) => match buffers.get_mut(&binding.get_buffer(CameraRender::CAMERA_IDX).unwrap()) {
            Some(buffer) => buffer,
            None => return
        },
        None => return
    };
    let render_instance = render_instance.0.read().unwrap();
    camera_buffer.buffer.write(
        &render_instance,
        bytemuck::cast_slice(&[camera_uniform.to_owned()]),
        0
    );
}
