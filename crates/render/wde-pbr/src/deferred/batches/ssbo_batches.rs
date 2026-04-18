use bevy::{ecs::system::SystemParamItem, prelude::*};
use wde_renderer::prelude::*;

use crate::prelude::BatchList;

/// The maximum number of batches in the ssbo.
pub const SSBO_MAX_BATCHES: usize = 100_000;

pub(crate) struct SsboInstancesToTransformPlugin;
impl Plugin for SsboInstancesToTransformPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderDataRegisterPlugin::<PbrSsboInstanceToTransform>::default());
    }
}

/// Contains pointers to the transforms of the entities in the SSBO, as well as the offset in the "instance to transform" buffer of the first entity of this batch. This is used to bind the correct transforms to the instances of each batch when rendering.
#[derive(Asset, Clone, TypePath, Default)]
pub struct PbrSsboInstanceToTransform;
impl PbrSsboInstanceToTransform {
    pub const INSTANCE_TO_TRANSFORM_IDX: u32 = 0;
    pub const INSTANCE_TO_TRANSFORM_STAGING_IDX: u32 = 1;
}
impl RenderData for PbrSsboInstanceToTransform {
    type Params = ();

    fn describe(_params: &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
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

pub(crate) fn set_instances_to_transform(
    instance_to_transform: ResRenderData<PbrSsboInstanceToTransform>,
    mut batches: ResMut<BatchList>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    render_instance: Res<RenderInstance>
) {
    if !batches.dirty {
        return;
    }

    // Get buffers
    let instance_to_transform = match instance_to_transform.iter().next() {
        Some((_, data)) => data,
        None => return
    };
    let (staging_buffer, gpu_buffer) = match (
        buffers.get(
            &instance_to_transform
                .get_buffer(PbrSsboInstanceToTransform::INSTANCE_TO_TRANSFORM_STAGING_IDX)
                .unwrap()
        ),
        buffers.get(
            &instance_to_transform
                .get_buffer(PbrSsboInstanceToTransform::INSTANCE_TO_TRANSFORM_IDX)
                .unwrap()
        )
    ) {
        (Some(staging), Some(gpu)) => (staging, gpu),
        _ => return
    };

    // Update the staging buffer with the new instance to transform mappings for each batch
    let mut staging_content = vec![0u8; staging_buffer.buffer.buffer.size() as usize];
    let mut offset = 0;
    for batch_key in batches.sorted_batches.clone().iter() {
        let batch = batches.batches.get_mut(batch_key).unwrap();
        batch.instances_offset = offset;
        for transform_id in &batch.transform_ssbo_ids {
            let bytes = transform_id.to_ne_bytes();
            staging_content[(offset as usize * 4)..((offset as usize + 1) * 4)]
                .copy_from_slice(&bytes);
            offset += 1;
        }
    }
    let render_instance = render_instance.0.read().unwrap();
    staging_buffer
        .buffer
        .write(&render_instance, &staging_content, 0);

    // Copy the staging buffer to the GPU buffer
    gpu_buffer
        .buffer
        .copy_from_buffer(&render_instance, &staging_buffer.buffer);

    // Clear dirty batches
    batches.dirty = false;
}
