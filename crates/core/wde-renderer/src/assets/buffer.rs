use super::asset::RenderAsset;
use crate::core::RenderInstance;
use bevy::{ecs::system::lifetimeless::SRes, prelude::*};
use wde_wgpu::buffer::{Buffer as WBuffer, BufferUsage as WBufferUsage};

/// Stores a CPU buffer with raw byte data and metadata for GPU allocation.
/// This buffer will be uploaded to the GPU, represented by a [`GpuBuffer`] asset.
#[derive(Asset, TypePath, Clone)]
pub struct Buffer {
    pub label: String,
    /// Size in bytes of the allocation.
    pub size: usize,
    /// Usage flags (storage, uniform, copy, etc.).
    pub usage: WBufferUsage,
    /// Optional initial payload copied to the GPU when present.
    pub content: Option<Vec<u8>>
}

/// Represents a GPU buffer resource allocated from a CPU [`Buffer`] asset.
pub struct GpuBuffer {
    pub label: String,
    /// Handle to the GPU buffer allocated via `wde-wgpu`.
    pub buffer: WBuffer
}
impl RenderAsset for GpuBuffer {
    type SourceAsset = Buffer;
    type Params = SRes<RenderInstance>;

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        render_instance: &mut bevy::ecs::system::SystemParamItem<Self::Params>
    ) -> Result<Self, super::asset::PrepareAssetError<Self::SourceAsset>> {
        let render_instance = render_instance.0.read().unwrap();
        let buffer = WBuffer::new(
            &render_instance,
            asset.label.as_str(),
            asset.size,
            asset.usage,
            asset.content.as_deref()
        );
        Ok(GpuBuffer {
            label: asset.label,
            buffer
        })
    }

    fn label(&self) -> &str {
        &self.label
    }
}
