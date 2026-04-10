use bevy::{ecs::system::SystemParamItem, prelude::*};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

/// The maximum number of entity transforms in the ssbo.
pub const SSBO_TRANSFORM_MAX_ENTITY: usize = 100_000;

pub(crate) struct SsboTransformPlugin;
impl Plugin for SsboTransformPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderDataRegisterPlugin::<PbrSsboTransform>::default());
    }
}

/// Contains the transform data for the entities in the scene.
/// See [crate::deferred] level documentation for more details on how this is used, and how it is updated.
#[derive(Asset, Clone, TypePath, Default)]
pub struct PbrSsboTransform;
impl PbrSsboTransform {
    pub const TRANSFORM_IDX: u32 = 0;
    pub const TRANSFORM_STAGING_IDX: u32 = 1;
}
impl RenderData for PbrSsboTransform {
    type Params = ();

    fn describe(_params: &SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        builder
            .add_buffer(
                Self::TRANSFORM_IDX,
                Buffer {
                    label: "pbr-ssbo-transform-gpu".to_string(),
                    size: std::mem::size_of::<TransformUniform>() * SSBO_TRANSFORM_MAX_ENTITY,
                    usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                    content: None
                }
            )
            .add_buffer(
                Self::TRANSFORM_STAGING_IDX,
                Buffer {
                    label: "pbr-ssbo-transform-staging".to_string(),
                    size: std::mem::size_of::<TransformUniform>() * SSBO_TRANSFORM_MAX_ENTITY,
                    usage: BufferUsage::COPY_SRC | BufferUsage::COPY_DST,
                    content: None
                }
            );
    }
}
