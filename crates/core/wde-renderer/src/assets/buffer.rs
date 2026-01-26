//! CPU and GPU buffer assets used by the renderer.
//!
//! The CPU [`Buffer`] mirrors the parameters accepted by `wde-wgpu::buffer` and
//! can optionally embed initial contents. [`GpuBuffer`] is created inside the
//! render app through the render assets pipeline.
//!
//! ## Example: create a dynamic uniform buffer
//! ```rust
//! use wde_renderer::assets::{Buffer, BufferUsage};
//!
//! // CPU-side asset definition (bytes aligned for uniform usage)
//! let buffer = Buffer {
//!     label: "camera-uniform".into(),
//!     size: 64,
//!     usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
//!     content: Some(vec![0u8; 64]),
//! };
//! // Add to AssetServer; render app will produce a `GpuBuffer` automatically.
//! ```

use bevy::{ecs::system::lifetimeless::SRes, prelude::*};
use wde_wgpu::{buffer::{BufferUsage as WBufferUsage, Buffer as WBuffer}};

use crate::core::RenderInstance;

use super::render_assets::RenderAsset;

// Reexport the buffer types
pub use wde_wgpu::buffer::{BufferUsage, BufferBindingType};
pub use wde_wgpu::command_buffer::*;

/// Stores a CPU buffer
#[derive(Asset, TypePath, Clone)]
pub struct Buffer {
    /// Debug label propagated to the GPU buffer.
    pub label: String,
    /// Size in bytes of the allocation.
    pub size: usize,
    /// Usage flags (storage, uniform, copy, etc.).
    pub usage: WBufferUsage,
    /// Optional initial payload copied to the GPU when present.
    pub content: Option<Vec<u8>>
}

/// Stores a GPU buffer
pub struct GpuBuffer {
    /// Copy of the CPU label applied to the GPU resource.
    pub label: String,
    /// Handle to the GPU buffer allocated via `wde-wgpu`.
    pub buffer: WBuffer
}
impl RenderAsset for GpuBuffer {
    type SourceAsset = Buffer;
    type Param = SRes<RenderInstance>;

    fn prepare_asset(
            asset: Self::SourceAsset,
            render_instance: &mut bevy::ecs::system::SystemParamItem<Self::Param>,
        ) -> Result<Self, super::render_assets::PrepareAssetError<Self::SourceAsset>> {
        let render_instance = render_instance.0.read().unwrap();
        let buffer = WBuffer::new(
            &render_instance,
            asset.label.as_str(),
            asset.size,
            asset.usage,
            asset.content.as_deref());
        Ok(GpuBuffer { label: asset.label, buffer })
    }

    fn label(&self) -> &str {
        &self.label
    }
}
