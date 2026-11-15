//! GPU resources shared across the renderer.
//!
//! - [`buffer`]: strongly-typed helpers around `wgpu::Buffer` for uploads, copies, and mapping.
//! - [`texture`]: 2D texture wrappers with ready-to-use views and samplers.
//! - [`memory`]: tracking of the GPU memory of the buffers and textures.
pub mod buffer;
pub mod memory;
pub mod texture;

pub use buffer::*;
pub use memory::*;
pub use texture::*;
