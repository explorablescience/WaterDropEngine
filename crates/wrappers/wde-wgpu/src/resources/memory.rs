//! Tracking of the GPU memory of the [`Buffer`](crate::buffer::Buffer)s and [`Texture`](crate::texture::Texture)s
//! (see [`GpuMemory`]).
//!
//! Sizes are estimated from the descriptors of the resources: the alignment, padding and compression of the driver
//! are not counted.

use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex, MutexGuard}
};

use wgpu::{BufferUsages, TextureFormat, TextureUsages};

/// Category of a tracked GPU resource, deduced from its usages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GpuMemoryCategory {
    /// Textures only sampled or copied, such as the loaded images.
    SampledTexture,
    RenderTarget,
    DepthStencil,
    StorageTexture,
    VertexBuffer,
    IndexBuffer,
    IndirectBuffer,
    StorageBuffer,
    UniformBuffer,
    /// Buffers mapped by the CPU, for uploads and readbacks.
    StagingBuffer,
    OtherBuffer
}
impl GpuMemoryCategory {
    pub const ALL: [Self; 11] = [
        Self::SampledTexture,
        Self::RenderTarget,
        Self::DepthStencil,
        Self::StorageTexture,
        Self::VertexBuffer,
        Self::IndexBuffer,
        Self::IndirectBuffer,
        Self::StorageBuffer,
        Self::UniformBuffer,
        Self::StagingBuffer,
        Self::OtherBuffer
    ];

    pub fn of_texture(format: TextureFormat, usage: TextureUsages) -> Self {
        if format.has_depth_aspect() || format.has_stencil_aspect() {
            Self::DepthStencil
        } else if usage.contains(TextureUsages::RENDER_ATTACHMENT) {
            Self::RenderTarget
        } else if usage.contains(TextureUsages::STORAGE_BINDING) {
            Self::StorageTexture
        } else {
            Self::SampledTexture
        }
    }

    pub fn of_buffer(usage: BufferUsages) -> Self {
        if usage.intersects(BufferUsages::MAP_READ | BufferUsages::MAP_WRITE) {
            Self::StagingBuffer
        } else if usage.contains(BufferUsages::INDIRECT) {
            Self::IndirectBuffer
        } else if usage.contains(BufferUsages::VERTEX) {
            Self::VertexBuffer
        } else if usage.contains(BufferUsages::INDEX) {
            Self::IndexBuffer
        } else if usage.contains(BufferUsages::STORAGE) {
            Self::StorageBuffer
        } else if usage.contains(BufferUsages::UNIFORM) {
            Self::UniformBuffer
        } else {
            Self::OtherBuffer
        }
    }

    pub fn is_texture(self) -> bool {
        matches!(
            self,
            Self::SampledTexture | Self::RenderTarget | Self::DepthStencil | Self::StorageTexture
        )
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::SampledTexture => "Sampled",
            Self::RenderTarget => "Render targets",
            Self::DepthStencil => "Depth / stencil",
            Self::StorageTexture => "Storage",
            Self::VertexBuffer => "Vertex",
            Self::IndexBuffer => "Index",
            Self::IndirectBuffer => "Indirect",
            Self::StorageBuffer => "Storage",
            Self::UniformBuffer => "Uniform",
            Self::StagingBuffer => "Staging",
            Self::OtherBuffer => "Other"
        }
    }
}

/// A live tracked GPU resource.
#[derive(Clone, Debug)]
pub struct GpuAllocationInfo {
    pub label: String,
    pub category: GpuMemoryCategory,
    pub bytes: u64,
    /// Size, format, layers, mips and samples of textures, empty for buffers.
    pub description: String
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GpuMemoryTotal {
    pub count: usize,
    pub bytes: u64
}

#[derive(Default)]
struct Registry {
    next_id: u64,
    allocations: HashMap<u64, GpuAllocationInfo>,
    totals: HashMap<GpuMemoryCategory, GpuMemoryTotal>
}
static REGISTRY: LazyLock<Mutex<Registry>> = LazyLock::new(Default::default);
fn registry() -> MutexGuard<'static, Registry> {
    REGISTRY.lock().unwrap_or_else(|e| e.into_inner())
}

/// A tracked GPU resource, untracked when dropped. Keep it next to the resource.
#[derive(Debug)]
pub struct GpuAllocation(u64);
impl GpuAllocation {
    pub fn track(info: GpuAllocationInfo) -> Self {
        let mut registry = registry();
        let id = registry.next_id;
        registry.next_id += 1;
        let total = registry.totals.entry(info.category).or_default();
        total.count += 1;
        total.bytes += info.bytes;
        registry.allocations.insert(id, info);
        Self(id)
    }

    pub fn buffer(label: &str, buffer: &wgpu::Buffer) -> Self {
        Self::track(GpuAllocationInfo {
            label: label.to_string(),
            category: GpuMemoryCategory::of_buffer(buffer.usage()),
            bytes: buffer.size(),
            description: String::new()
        })
    }

    pub fn texture(label: &str, texture: &wgpu::Texture) -> Self {
        let size = texture.size();
        let (format, layers, mips, samples) = (
            texture.format(),
            size.depth_or_array_layers,
            texture.mip_level_count(),
            texture.sample_count()
        );
        let mut description = format!("{}x{} {format:?}", size.width, size.height);
        for (count, unit) in [(layers, " layers"), (mips, " mips"), (samples, "x MSAA")] {
            if count > 1 {
                description += &format!(", {count}{unit}");
            }
        }
        Self::track(GpuAllocationInfo {
            label: label.to_string(),
            category: GpuMemoryCategory::of_texture(format, texture.usage()),
            bytes: texture_bytes(format, (size.width, size.height), layers, mips, samples),
            description
        })
    }
}
impl GpuAllocation {
    /// Overrides the category deduced from the usages.
    pub fn set_category(&self, category: GpuMemoryCategory) {
        let mut registry = registry();
        let Some(info) = registry.allocations.get_mut(&self.0) else {
            return;
        };
        let (previous, bytes) = (std::mem::replace(&mut info.category, category), info.bytes);
        if previous == category {
            return;
        }
        if let Some(total) = registry.totals.get_mut(&previous) {
            total.count -= 1;
            total.bytes -= bytes;
        }
        let total = registry.totals.entry(category).or_default();
        total.count += 1;
        total.bytes += bytes;
    }
}
impl Drop for GpuAllocation {
    fn drop(&mut self) {
        let mut registry = registry();
        let Some(info) = registry.allocations.remove(&self.0) else {
            return;
        };
        if let Some(total) = registry.totals.get_mut(&info.category) {
            total.count -= 1;
            total.bytes -= info.bytes;
        }
    }
}

/// GPU memory of the live [`Buffer`](crate::buffer::Buffer)s and [`Texture`](crate::texture::Texture)s, shared
/// by the whole process. Resources created directly with `wgpu` are only counted if they hold a [`GpuAllocation`].
pub struct GpuMemory;
impl GpuMemory {
    /// Totals of every category, in the order of [`GpuMemoryCategory::ALL`].
    pub fn totals() -> Vec<(GpuMemoryCategory, GpuMemoryTotal)> {
        let registry = registry();
        GpuMemoryCategory::ALL
            .iter()
            .map(|c| (*c, registry.totals.get(c).copied().unwrap_or_default()))
            .collect()
    }

    pub fn allocations() -> Vec<GpuAllocationInfo> {
        registry().allocations.values().cloned().collect()
    }
}

/// Estimated size in bytes of a 2D texture with all its mips, layers and samples.
pub fn texture_bytes(
    format: TextureFormat,
    size: (u32, u32),
    layers: u32,
    mips: u32,
    samples: u32
) -> u64 {
    let (block_width, block_height) = format.block_dimensions();
    let block_bytes = match format {
        TextureFormat::Depth24Plus | TextureFormat::Depth24PlusStencil8 => 4,
        TextureFormat::Depth32FloatStencil8 => 8,
        f => f.block_copy_size(None).unwrap_or(4)
    } as u64;
    let layer_bytes: u64 = (0..mips.max(1))
        .map(|mip| {
            let width = (size.0 >> mip).max(1).div_ceil(block_width) as u64;
            let height = (size.1 >> mip).max(1).div_ceil(block_height) as u64;
            width * height * block_bytes
        })
        .sum();
    layer_bytes * layers.max(1) as u64 * samples.max(1) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_sizes() {
        assert_eq!(
            texture_bytes(TextureFormat::Rgba8Unorm, (256, 256), 1, 1, 1),
            262_144
        );
        // 256² + 128² + ... + 1² texels
        assert_eq!(
            texture_bytes(TextureFormat::R8Unorm, (256, 256), 1, 9, 1),
            87_381
        );
        assert_eq!(
            texture_bytes(TextureFormat::Bc7RgbaUnorm, (256, 256), 1, 1, 1),
            65_536
        );
        assert_eq!(
            texture_bytes(TextureFormat::Bc1RgbaUnorm, (2, 2), 1, 1, 1),
            8
        );
        assert_eq!(
            texture_bytes(TextureFormat::Depth32Float, (16, 16), 6, 1, 4),
            24_576
        );
    }

    #[test]
    fn tracking() {
        let total = || {
            GpuMemory::totals()
                .into_iter()
                .find(|(c, _)| *c == GpuMemoryCategory::OtherBuffer)
                .unwrap()
                .1
        };
        let info = |bytes| GpuAllocationInfo {
            label: "test".to_string(),
            category: GpuMemoryCategory::OtherBuffer,
            bytes,
            description: String::new()
        };
        let before = total();
        let a = GpuAllocation::track(info(100));
        let b = GpuAllocation::track(info(20));
        assert_eq!(total().bytes, before.bytes + 120);
        drop(a);
        assert_eq!(total().count, before.count + 1);
        b.set_category(GpuMemoryCategory::StagingBuffer);
        assert_eq!(total().count, before.count);
        drop(b);
        assert_eq!(total(), before);
    }

    #[test]
    fn categories() {
        use GpuMemoryCategory as C;
        let texture = |format, usage| C::of_texture(format, usage);
        assert_eq!(
            texture(
                TextureFormat::Depth32Float,
                TextureUsages::RENDER_ATTACHMENT
            ),
            C::DepthStencil
        );
        assert_eq!(
            texture(TextureFormat::Rgba16Float, TextureUsages::RENDER_ATTACHMENT),
            C::RenderTarget
        );
        assert_eq!(
            C::of_buffer(BufferUsages::STORAGE | BufferUsages::INDIRECT),
            C::IndirectBuffer
        );
        assert_eq!(
            C::of_buffer(BufferUsages::COPY_DST | BufferUsages::MAP_READ),
            C::StagingBuffer
        );
    }
}
