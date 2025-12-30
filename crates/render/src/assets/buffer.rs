use bevy::{ecs::system::lifetimeless::SRes, prelude::*};
use wde_wgpu::{buffer::{BufferUsage, Buffer as WBuffer}, instance::RenderInstance};

use super::render_assets::RenderAsset;

/// Stores a CPU buffer
#[derive(Asset, TypePath, Clone)]
pub struct Buffer {
    pub label: String,
    pub size: usize,
    pub usage: BufferUsage,
    pub content: Option<Vec<u8>>
}

/// Stores a GPU buffer
pub struct GpuBuffer {
    pub label: String,
    pub buffer: WBuffer
}
impl RenderAsset for GpuBuffer {
    type SourceAsset = Buffer;
    type Param = SRes<RenderInstance<'static>>;

    fn prepare_asset(
            asset: Self::SourceAsset,
            render_instance: &mut bevy::ecs::system::SystemParamItem<Self::Param>,
        ) -> Result<Self, super::render_assets::PrepareAssetError<Self::SourceAsset>> {
        let render_instance = render_instance.data.read().unwrap();
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
