use bevy::{ecs::system::SystemParamItem, prelude::*};
use wde_renderer::prelude::*;

use crate::view::{CameraUniform, CameraView};

pub(crate) struct CameraFeature;
impl Plugin for CameraFeature {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RenderDataRegisterPlugin::<CameraRender>::default(),
            RenderBindingRegisterPlugin::<CameraBinding>::default()
        ));
        app.init_resource::<ProjectionJitter>();
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

/// Marker component for the active camera, rendered on screen. The render feature reads this each frame to know which camera to use for rendering.
#[derive(Component, Default, Clone, Debug, Reflect)]
#[reflect(Component)]
#[require(Transform, CameraView, Camera)]
pub struct ActiveCamera;

/// Offsets the projection of the [`ActiveCamera`] by a different sub-pixel amount each frame, cycling over
/// [`ProjectionJitter::SEQUENCE_LENGTH`] frames, for temporal anti-aliasing. The `view_to_ndc` and `ndc_to_view`
/// matrices of the camera uniform are jittered, and its `world_to_ndc_unjittered` and `prev_world_to_ndc` are not.
#[derive(Resource, Clone, Debug, Default)]
pub struct ProjectionJitter {
    pub enabled: bool
}
impl ProjectionJitter {
    pub const SEQUENCE_LENGTH: u32 = 8;

    /// Offset of the `frame`, in pixels within [-0.5, 0.5], from the Halton (2, 3) sequence.
    pub fn offset(frame: u32) -> Vec2 {
        let index = frame % Self::SEQUENCE_LENGTH + 1;
        Vec2::new(halton(index, 2), halton(index, 3)) - 0.5
    }
}

fn halton(mut index: u32, base: u32) -> f32 {
    let (mut result, mut fraction) = (0.0, 1.0);
    while index > 0 {
        fraction /= base as f32;
        result += fraction * (index % base) as f32;
        index /= base;
    }
    result
}

/// Camera render data buffer. This is used to store the camera uniform data that will be sent to the GPU each frame.
#[derive(Asset, Clone, TypePath, Default)]
pub struct CameraRender;
impl CameraRender {
    pub const CAMERA_IDX: u32 = 0;
}
impl RenderData for CameraRender {
    type Params = ();

    fn describe(_params: &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
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

    fn describe(
        &mut self,
        camera_render: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder.add_buffer(camera_render, CameraRender::CAMERA_IDX);
    }

    fn label(&self) -> &'static str {
        "camera"
    }
}

fn extract(
    cameras: ExtractWorld<Query<(&GlobalTransform, &CameraView), With<ActiveCamera>>>,
    jitter: ExtractWorld<Res<ProjectionJitter>>,
    mut camera_uniform: ResMut<CameraUniform>,
    window: ExtractWorld<Query<&Window>>,
    mut frame: Local<u32>,
    mut prev_world_to_ndc: Local<Option<Mat4>>
) {
    // Update the camera uniform
    if let (Ok((transform, view)), Ok(window)) = (cameras.single(), window.single()) {
        let aspect_ratio = window.width() / window.height();
        let ndc_jitter = match jitter.enabled {
            true => {
                *frame = frame.wrapping_add(1);
                let size = Vec2::new(
                    window.resolution.physical_width().max(1) as f32,
                    window.resolution.physical_height().max(1) as f32
                );
                ProjectionJitter::offset(*frame) * 2.0 / size
            }
            false => Vec2::ZERO
        };
        let world_to_ndc = view.world_to_ndc(transform, aspect_ratio);
        *camera_uniform = CameraUniform::new(
            transform,
            view,
            aspect_ratio,
            ndc_jitter,
            prev_world_to_ndc.unwrap_or(world_to_ndc)
        );
        *prev_world_to_ndc = Some(world_to_ndc);
    }
}

fn update_buffer(
    render_instance: Res<RenderInstance>,
    camera_uniform: Res<CameraUniform>,
    camera_buffer: ResRenderData<CameraRender>,
    mut buffers: ResMut<RenderAssets<GpuBuffer>>
) {
    let camera_buffer = match camera_buffer.iter().next() {
        Some((_, binding)) => {
            match buffers.get_mut(&binding.get_buffer(CameraRender::CAMERA_IDX).unwrap()) {
                Some(buffer) => buffer,
                None => return
            }
        }
        None => return
    };
    let render_instance = render_instance.0.read().unwrap();
    camera_buffer.buffer.write(
        &render_instance,
        bytemuck::cast_slice(&[camera_uniform.to_owned()]),
        0
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jitter_offsets_are_distinct_and_centered() {
        let offsets: Vec<Vec2> = (0..ProjectionJitter::SEQUENCE_LENGTH)
            .map(ProjectionJitter::offset)
            .collect();
        for (i, a) in offsets.iter().enumerate() {
            assert!(a.abs().max_element() <= 0.5, "{a}");
            assert!(offsets[i + 1..].iter().all(|b| a.distance(*b) > 0.05));
        }
        let mean = offsets.iter().sum::<Vec2>() / offsets.len() as f32;
        assert!(mean.abs().max_element() < 0.1, "{mean}");
        assert_eq!(
            ProjectionJitter::offset(0),
            ProjectionJitter::offset(ProjectionJitter::SEQUENCE_LENGTH)
        );
    }
}
