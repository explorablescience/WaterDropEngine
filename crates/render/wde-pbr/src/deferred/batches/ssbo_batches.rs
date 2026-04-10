use bevy::{ecs::system::SystemParamItem, prelude::*};
use wde_renderer::prelude::*;

/// The maximum number of batches in the ssbo.
pub const SSBO_MAX_BATCHES: usize = 100_000;

pub(crate) struct SsboInstancesToTransformPlugin;
impl Plugin for SsboInstancesToTransformPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderDataRegisterPlugin::<PbrSsboInstanceToTransform>::default());
    }
}

/// Contains the transform data for the entities in the scene.
/// See [crate::deferred] level documentation for more details on how this is used, and how it is updated.
#[derive(Asset, Clone, TypePath, Default)]
pub struct PbrSsboInstanceToTransform;
impl PbrSsboInstanceToTransform {
    pub const INSTANCE_TO_TRANSFORM_IDX: u32 = 0;
    pub const INSTANCE_TO_TRANSFORM_STAGING_IDX: u32 = 1;
}
impl RenderData for PbrSsboInstanceToTransform {
    type Params = ();

    fn describe(_params: &SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        builder
            .add_buffer(
                Self::INSTANCE_TO_TRANSFORM_IDX,
                Buffer {
                    label: "pbr-instance-to-transform-buffer-gpu".to_string(),
                    size: std::mem::size_of::<u32>() * SSBO_MAX_BATCHES,
                    usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                    content: None
                }
            )
            .add_buffer(
                Self::INSTANCE_TO_TRANSFORM_STAGING_IDX,
                Buffer {
                    label: "pbr-instance-to-transform-staging".to_string(),
                    size: std::mem::size_of::<u32>() * SSBO_MAX_BATCHES,
                    usage: BufferUsage::COPY_SRC | BufferUsage::COPY_DST,
                    content: None
                }
            );
    }
}
