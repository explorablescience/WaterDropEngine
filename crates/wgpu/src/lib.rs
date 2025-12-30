//! WaterDropEngine's `wde-wgpu` crate is a lightweight layer over `wgpu` that keeps the
//! low-level power of WebGPU while offering opinionated building blocks for buffers,
//! textures, render/compute pipelines, and pass orchestration. All submodules are
//! re-exported at the crate root (e.g. `wde_wgpu::buffer`, `wde_wgpu::command_buffer`)
//! to keep imports short and examples consistent.
//!
//! # Architecture
//! - **Instance and surface**: [`instance::create_instance`] produces a [`instance::RenderInstance`]
//!   holding the `wgpu::Device`, `Queue`, and optional `Surface`. [`instance::setup_surface`] stores
//!   a [`wgpu::SurfaceConfiguration`] once the window size is known and [`instance::resize`]
//!   reapplies it on swapchain loss or window resize.
//! - **Resources**: [`buffer::Buffer`] and [`texture::Texture`] wrap GPU memory and add
//!   convenience copy helpers plus defaults for common usages (render targets, depth).
//! - **Pipelines**: [`render_pipeline::RenderPipeline`] and [`compute_pipeline::ComputePipeline`]
//!   compile WGSL source, build pipeline layouts (bind groups + push constants), and expose
//!   `is_initialized` + getters for the underlying `wgpu` objects.
//! - **Pass orchestration**: [`command_buffer::CommandBuffer`] records GPU work. It spawns
//!   [`render_pass::RenderPass`] and [`compute_pass::WComputePass`] with guard rails that
//!   check for missing pipelines/buffers before issuing draws or dispatches.
//! - **Bind groups**: [`bind_group`] helps assemble layouts and bind groups that match WGSL
//!   declarations.
//! - **Vertex helpers**: [`vertex::Vertex`] provides a canonical position/uv/normal layout
//!   and a ready-to-use `wgpu::VertexBufferLayout`.
//!
//! # Quickstart (hello clear)
//! ```rust,no_run
//! use bevy::window::RawHandleWrapperHolder;
//! use wde_wgpu::{
//!     command_buffer::{CommandBuffer, Operations, RenderPassBuilder, RenderPassColorAttachment},
//!     instance::{create_instance, get_current_texture, present, setup_surface, PresentMode, RenderEvent},
//! };
//!
//! # async fn demo(window_handle: RawHandleWrapperHolder, size: (u32, u32)) {
//! let render = create_instance("hello-wde", Some(&window_handle)).await;
//!
//! // Configure the surface once the window is ready
//! {
//!     let mut data = render.data.write().unwrap();
//!     let surface = data.surface.as_ref().unwrap();
//!     data.surface_config = Some(setup_surface(
//!         "main-surface",
//!         size,
//!         &data.device,
//!         surface,
//!         &data.adapter,
//!         PresentMode::AutoNoVsync,
//!     ));
//! }
//!
//! // Acquire the next frame and record work
//! let render_event = {
//!     let data = render.data.read().unwrap();
//!     get_current_texture(
//!         data.surface.as_ref().unwrap(),
//!         data.surface_config.as_ref().unwrap(),
//!     )
//! };
//!
//! if let RenderEvent::Redraw(frame) = render_event {
//!     let data = render.data.read().unwrap();
//!     let mut cmd = CommandBuffer::new(&data, "clear-pass");
//!     {
//!         cmd.create_render_pass("clear", |pass| {
//!             pass.add_color_attachment(RenderPassColorAttachment {
//!                 texture: Some(&frame.view),
//!                 load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
//!                 store: wgpu::StoreOp::Store,
//!             });
//!         });
//!     }
//!     cmd.submit(&data);
//!     present(frame.texture).unwrap();
//! }
//! # }
//! ```
//!
//! # Core usage patterns
//! - Configure a surface once, then react to [`instance::RenderEvent::Resize`] with [`instance::resize`].
//! - Build resources up-front (`Buffer`, `Texture`), then stage per-frame uploads via
//!   [`buffer::Buffer::write`] or `queue.write_texture` wrappers.
//! - Keep pipeline creation deterministic: set shaders, bind group layouts, push constants,
//!   then call `init` and gate draws/dispatches on `is_initialized`.
//! - Record work inside a `CommandBuffer`; each render/compute pass enforces that required
//!   state (pipeline, vertex/index buffers) is set before issuing commands.
//!
//! # Modules
//! - [`instance`]: device, queue, surface, events, presentation helpers.
//! - [`buffer`] / [`texture`]: GPU buffers and 2D textures with helper views/samplers.
//! - [`render_pipeline`] / [`compute_pipeline`]: WGSL compilation + pipeline layout creation.
//! - [`render_pass`] / [`compute_pass`] / [`command_buffer`]: pass-level helpers with safety checks.
//! - [`bind_group`]: bind group layout builders and bind group creation.
//! - [`vertex`]: canonical vertex layout used across examples.
//!
//! # Examples and further reading
//! - Minimal WGSL shaders live in `res/` (see `res/examples` for full scenes).
//! - Bind group helpers sit in [`bind_group`]; pair them with pipeline layouts for material
//!   or compute resource binding.
//! - For GPU-driven draws, see indirect helpers on [`render_pass::RenderPass`].
//! - The examples under `res/examples` show complete scenes; start with `display_texture` for
//!   a minimal textured quad.
pub mod instance;
pub mod resources;
pub mod pipelines;
pub mod passes;
pub mod utils;

// Re-exports to provide `wde_wgpu::buffer`, `wde_wgpu::command_buffer`, etc.
pub use instance::RenderInstance;
pub use resources::{buffer, texture};
pub use passes::{command_buffer, compute_pass, render_pass};
pub use pipelines::{bind_group, compute_pipeline, render_pipeline};
pub use utils::vertex;
