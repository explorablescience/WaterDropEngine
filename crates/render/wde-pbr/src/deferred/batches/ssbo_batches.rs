use bevy::{ecs::system::SystemParamItem, prelude::*};
use wde_renderer::prelude::*;

use crate::prelude::CULL_VIEW_COUNT;

/// The maximum number of batched instances visible in each culling view.
pub const SSBO_MAX_BATCHES: usize = 100_000;

pub(crate) struct SsboInstancesToTransformPlugin;
impl Plugin for SsboInstancesToTransformPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderDataRegisterPlugin::<PbrSsboInstanceToTransform>::default());
    }
}

/// Contains pointers to the transforms of the entities in the SSBO: for each culling view, the visible instances of each batch, contiguous. This is used to bind the correct transforms to the instances of each batch when rendering.
#[derive(Asset, Clone, TypePath, Default)]
pub struct PbrSsboInstanceToTransform;
impl PbrSsboInstanceToTransform {
    pub const INSTANCE_TO_TRANSFORM_IDX: u32 = 0;
}
impl RenderData for PbrSsboInstanceToTransform {
    type Params = ();

    fn describe(_params: &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        builder.add_buffer(
            Self::INSTANCE_TO_TRANSFORM_IDX,
            Buffer {
                label: "pbr-instance-to-transform-buffer-gpu".to_string(),
                size: std::mem::size_of::<u32>() * SSBO_MAX_BATCHES * CULL_VIEW_COUNT,
                usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                content: None
            }
        );
    }
}
