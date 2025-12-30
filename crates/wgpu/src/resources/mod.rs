//! GPU resources shared across the renderer.
//!
//! - [`buffer`]: strongly-typed helpers around `wgpu::Buffer` for uploads, copies, and mapping.
//! - [`texture`]: 2D texture wrappers with ready-to-use views and samplers.
pub mod buffer;
pub mod texture;

pub use buffer::*;
pub use texture::*;
