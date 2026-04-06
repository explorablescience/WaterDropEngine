use bevy::prelude::*;
use wde_renderer::prelude::*;

/// The maximum number of entity transforms in the ssbo.
pub const SSBO_TRANSFORM_MAX_ENTITY: usize = 100_000;

pub(crate) struct SsboTransformPlugin;
impl Plugin for SsboTransformPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RenderBindingPluginRegister::<SsboTransform>::default(),
            RenderBindingPluginRegister::<SsboTransformStaging>::default()
        ));
    }
}

/// Contains the transform data for the entities in the scene.
/// See [crate::deferred] level documentation for more details on how this is used, and how it is updated.
#[derive(Asset, Clone, TypePath, Default)]
pub struct SsboTransform;
impl SsboTransform {
    pub const SSBO_ID: u32 = 0;
    pub const INSTANCE_TO_TRANSFORM_ID: u32 = 1;
}
impl RenderBinding for SsboTransform {
    fn describe(&self, builder: &mut RenderBindingBuilder) {
        builder.add_buffer(
            Self::SSBO_ID,
            Buffer {
                label: "pbr-ssbo-transform-gpu".to_string(),
                size: std::mem::size_of::<TransformUniform>() * SSBO_TRANSFORM_MAX_ENTITY,
                usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                content: None
            }
        );
        builder.add_buffer(
            Self::INSTANCE_TO_TRANSFORM_ID,
            Buffer {
                label: "pbr-instance-to-transform-buffer-gpu".to_string(),
                size: std::mem::size_of::<u32>() * SSBO_TRANSFORM_MAX_ENTITY,
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
pub struct SsboTransformStaging;
impl SsboTransformStaging {
    pub const SSBO_STAGING_ID: u32 = 0;
    pub const INSTANCE_TO_TRANSFORM_STAGING_ID: u32 = 1;
}
impl RenderBinding for SsboTransformStaging {
    fn describe(&self, builder: &mut RenderBindingBuilder) {
        builder.add_buffer(
            Self::SSBO_STAGING_ID,
            Buffer {
                label: "pbr-ssbo-transform-staging".to_string(),
                size: std::mem::size_of::<TransformUniform>() * SSBO_TRANSFORM_MAX_ENTITY,
                usage: BufferUsage::COPY_SRC | BufferUsage::COPY_DST,
                content: None
            }
        );
        builder.add_buffer(
            Self::INSTANCE_TO_TRANSFORM_STAGING_ID,
            Buffer {
                label: "pbr-instance-to-transform-staging".to_string(),
                size: std::mem::size_of::<u32>() * SSBO_TRANSFORM_MAX_ENTITY,
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
