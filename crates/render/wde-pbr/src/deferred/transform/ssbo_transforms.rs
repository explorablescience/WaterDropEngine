use bevy::prelude::*;
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

/// The maximum number of entity transforms in the ssbo.
pub const SSBO_TRANSFORM_MAX_ENTITY: usize = 100_000;

pub(crate) struct SsboTransformPlugin;
impl Plugin for SsboTransformPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RenderBindingPluginRegister::<PbrSsboTransform>::default(),
            RenderBindingPluginRegister::<PbrSsboTransformStaging>::default()
        ));
    }
}

/// Contains the transform data for the entities in the scene.
/// See [crate::deferred] level documentation for more details on how this is used, and how it is updated.
#[derive(Asset, Clone, TypePath, Default)]
pub struct PbrSsboTransform;
impl RenderBinding for PbrSsboTransform {
    fn describe(&self, builder: &mut RenderBindingBuilder) {
        builder.add_buffer(
            0,
            Buffer {
                label: "pbr-ssbo-transform-gpu".to_string(),
                size: std::mem::size_of::<TransformUniform>() * SSBO_TRANSFORM_MAX_ENTITY,
                usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                content: None
            }
        );
    }

    fn label(&self) -> &'static str {
        "pbr-ssbo-transform"
    }
}

#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct PbrSsboTransformStaging;
impl RenderBinding for PbrSsboTransformStaging {
    fn describe(&self, builder: &mut RenderBindingBuilder) {
        builder.add_buffer(
            0,
            Buffer {
                label: "pbr-ssbo-transform-staging".to_string(),
                size: std::mem::size_of::<TransformUniform>() * SSBO_TRANSFORM_MAX_ENTITY,
                usage: BufferUsage::COPY_SRC | BufferUsage::COPY_DST,
                content: None
            }
        );

        // As this is a pure staging cpu side buffer, don't need a bind group
        builder.no_bind_group();
    }

    fn label(&self) -> &'static str {
        "pbr-ssbo-transform-staging"
    }
}
