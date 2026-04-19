//! Command encoder utilities to record render and compute work.
use wgpu::Texture;

use crate::{
    buffer::Buffer, compute_pass::WComputePass, instance::RenderInstanceData, texture::TextureView
};

use super::render_pass::RenderPassInstance;

/// Type of a color.
pub type WgpuColor = wgpu::Color;

/// Type of a load operation.
pub type LoadOp<V> = wgpu::LoadOp<V>;

/// Type of a store operation.
pub type StoreOp = wgpu::StoreOp;

/// Load/store operations for attachments.
#[derive(Clone, Copy, Debug)]
pub struct Operations<V> {
    pub load: LoadOp<V>,
    pub store: StoreOp
}

/// Describe the optional depth attachment of a render pass.
pub struct RenderPassDepth<'pass> {
    /// The depth texture. If `None`, the render pass will not have a depth texture.
    pub texture: Option<&'pass TextureView>,
    /// The depth operation when loading the texture. By default, load the existing texture.
    pub load: LoadOp<f32>,
    /// The depth operation when storing the texture. By default, store the texture.
    pub store: StoreOp,
    /// The stencil operation when loading the stencil buffer. By default, load the existing stencil content.
    pub stencil_load: LoadOp<u32>,
    /// The stencil operation when storing the stencil buffer. By default, store the stencil content.
    pub stencil_store: StoreOp
}
impl Default for RenderPassDepth<'_> {
    fn default() -> Self {
        Self {
            texture: None,
            // load: wgpu::LoadOp::Clear(1.0),
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
            stencil_load: wgpu::LoadOp::Load,
            stencil_store: wgpu::StoreOp::Store
        }
    }
}

/// Describe a single color attachment of a render pass.
pub struct RenderPassColorAttachment<'pass> {
    /// The color texture.
    pub texture: Option<&'pass TextureView>,
    /// The color load operation. By default, load the existing texture.
    pub load: LoadOp<WgpuColor>,
    /// The color store operation. By default, store the texture.
    pub store: StoreOp,
    /// An optional resolve target for multi-sampled textures.
    /// This will resolve the multi-sampled texture into the single-sampled target at the end of the render pass, when using multi-sampling.
    pub resolve_target: Option<&'pass TextureView>
}
impl Default for RenderPassColorAttachment<'_> {
    fn default() -> Self {
        Self {
            texture: None,
            // load: wgpu::LoadOp::Clear(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
            resolve_target: None
        }
    }
}

/// Builder for a render pass.
///
/// Prefer constructing passes through [`CommandBuffer::create_render_pass`]; supply a closure
/// that mutates this builder to add one or more color attachments and an optional depth
/// attachment.
#[derive(Default)]
pub struct RenderPassBuilder<'pass> {
    /// The depth texture of the render pass.
    depth: RenderPassDepth<'pass>,
    /// The color attachments of the render pass. By default, no color attachments.
    color_attachments: Vec<RenderPassColorAttachment<'pass>>
}
impl<'pass> RenderPassBuilder<'pass> {
    /// Set the depth texture of the render pass.
    ///
    /// # Arguments
    ///
    /// * `texture` - The depth texture.
    pub fn set_depth_texture(&mut self, texture: RenderPassDepth<'pass>) {
        self.depth = texture;
    }

    /// Add a color attachment to the render pass.
    ///
    /// # Arguments
    ///
    /// * `attachment` - The color attachment.
    pub fn add_color_attachment(&mut self, attachment: RenderPassColorAttachment<'pass>) {
        self.color_attachments.push(attachment);
    }
}

/// Record commands for the GPU, spawn render/compute passes, and submit once finished.
/// Guard rails ensure pipelines/buffers are set before draws/dispatches.
///
/// # Examples
/// Clear a surface:
/// ```rust,no_run
/// use wde_wgpu::{
///     command_buffer::{Color, CommandBuffer, LoadOp, RenderPassColorAttachment, StoreOp},
///     instance::RenderInstanceData,
///     texture::TextureView,
/// };
///
/// let mut cmd = CommandBuffer::new(instance, "frame");
/// {
///     cmd.create_render_pass("clear", |pass| {
///         pass.add_color_attachment(RenderPassColorAttachment {
///             texture: Some(view),
///             load: LoadOp::Clear(Color::BLACK),
///             store: StoreOp::Store,
///         });
///     });
/// }
/// cmd.submit(instance);
/// ```
///
/// Dispatch compute work:
/// ```rust,no_run
/// use wde_wgpu::{command_buffer::CommandBuffer, compute_pass::WComputePass, compute_pipeline::ComputePipeline};
///
/// let mut cmd = CommandBuffer::new(instance, "compute");
/// {
///     let mut pass: WComputePass = cmd.create_compute_pass("cull");
///     pass.set_pipeline(pipeline).unwrap().set_bind_group(0, bind).dispatch(4, 1, 1).unwrap();
/// }
/// cmd.submit(instance);
/// ```
pub struct CommandBuffer {
    pub label: String,
    encoder: wgpu::CommandEncoder
}

impl std::fmt::Debug for CommandBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandBuffer")
            .field("label", &self.label)
            .finish()
    }
}

impl CommandBuffer {
    /// Create a new command buffer.
    ///
    /// # Arguments
    ///
    /// * `instance` - The render instance.
    /// * `label` - The label of the command buffer.
    pub fn new(instance: &RenderInstanceData<'_>, label: &str) -> Self {
        // Create command encoder
        let command_encoder =
            instance
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some(format!("{}-command-encoder", label).as_str())
                });

        Self {
            label: label.to_string(),
            encoder: command_encoder
        }
    }

    /// Create a new render pass.
    ///
    /// # Arguments
    ///
    /// * `label` - The label of the render pass.
    /// * `builder_func` - The function to build the render pass. This function should return `Ok(())` if the render pass was built successfully, and `Err(String)` otherwise. If the function returns `Err`, the render pass will not be created.
    pub fn create_render_pass<'pass>(
        &'pass mut self,
        label: &str,
        builder_func: impl FnOnce(&mut RenderPassBuilder<'pass>) -> Result<(), String>
    ) -> Result<RenderPassInstance<'pass>, String> {
        // Run the builder function
        let mut builder = RenderPassBuilder::default();
        builder_func(&mut builder)?;

        let mut depth_attachment = None;
        if let Some(depth_texture) = builder.depth.texture {
            depth_attachment = Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_texture,
                depth_ops: Some(wgpu::Operations {
                    load: builder.depth.load,
                    store: builder.depth.store
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: builder.depth.stencil_load,
                    store: builder.depth.stencil_store
                })
            });
        }

        let color_attachments = builder
            .color_attachments
            .iter()
            .map(|attachment| {
                attachment
                    .texture
                    .map(|texture| wgpu::RenderPassColorAttachment {
                        view: texture,
                        resolve_target: attachment.resolve_target,
                        ops: wgpu::Operations {
                            load: attachment.load,
                            store: attachment.store
                        },
                        depth_slice: None
                    })
            })
            .collect::<Vec<_>>();

        let render_pass = self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(format!("{}-render-pass", label).as_str()),
            color_attachments: &color_attachments,
            depth_stencil_attachment: depth_attachment,
            timestamp_writes: None,
            occlusion_query_set: None
        });

        Ok(RenderPassInstance::new(label, render_pass))
    }

    /// Create a new compute pass.
    ///
    /// # Arguments
    ///
    /// * `label` - The label of the compute pass.
    pub fn create_compute_pass<'pass>(&'pass mut self, label: &str) -> WComputePass<'pass> {
        let compute_pass = self
            .encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(format!("{}-compute-pass", label).as_str()),
                timestamp_writes: None
            });

        WComputePass::new(label, compute_pass)
    }

    /// Finish and submit a command buffer.
    ///
    /// # Arguments
    ///
    /// * `instance` - The render instance.
    pub fn submit(self, instance: &RenderInstanceData) {
        instance
            .queue
            .submit(std::iter::once(self.encoder.finish()));
    }

    /// Copy a buffer to another buffer.
    /// Please use the `copy_from_buffer` method of the buffer to copy data.
    ///
    /// # Arguments
    ///
    /// * `source` - The source buffer.
    /// * `destination` - The destination buffer.
    pub fn copy_buffer_to_buffer(&mut self, source: &Buffer, destination: &Buffer) {
        self.encoder.copy_buffer_to_buffer(
            &source.buffer,
            0,
            &destination.buffer,
            0,
            source.buffer.size()
        );
    }

    /// Copy a buffer to another buffer with offsets.
    /// Please use the `copy_from_buffer` method of the buffer to copy data.
    ///
    /// # Arguments
    ///
    /// * `source` - The source buffer.
    /// * `destination` - The destination buffer.
    /// * `source_offset` - The offset in the source buffer.
    /// * `destination_offset` - The offset in the destination buffer.
    /// * `size` - The size to copy.
    pub fn copy_buffer_to_buffer_offset(
        &mut self,
        source: &Buffer,
        destination: &Buffer,
        source_offset: u64,
        destination_offset: u64,
        size: u64
    ) {
        self.encoder.copy_buffer_to_buffer(
            &source.buffer,
            source_offset,
            &destination.buffer,
            destination_offset,
            size
        );
    }

    /// Copy a texture to a buffer.
    /// Please use the `copy_from_texture` method of the buffer to copy data.
    ///
    /// # Arguments
    ///
    /// * `source` - The source texture.
    /// * `destination` - The destination buffer.
    /// * `size` - The size of the texture.
    pub fn copy_texture_to_buffer(
        &mut self,
        source: &Texture,
        destination: &Buffer,
        size: wgpu::Extent3d
    ) {
        // Create texture copy
        let texture_copy = source.as_image_copy();

        // Calculate bytes per row (must be a multiple of 256 for wgpu)
        let bytes_per_pixel = match source.format() {
            wgpu::TextureFormat::R8Unorm => 1,
            wgpu::TextureFormat::Rgba8Unorm => 4,
            _ => panic!("Unsupported texture format for copy")
        };
        let bytes_per_row = (size.width * bytes_per_pixel).div_ceil(256) * 256; // Align to 256 bytes

        // Create buffer copy
        let buffer_copy = wgpu::TexelCopyBufferInfo {
            buffer: &destination.buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: None
            }
        };

        // Copy texture to buffer
        self.encoder
            .copy_texture_to_buffer(texture_copy, buffer_copy, size);
    }

    /// Get the encoder of the command buffer.
    ///
    /// # Returns
    ///
    /// The encoder of the command buffer.
    pub fn encoder(&mut self) -> &mut wgpu::CommandEncoder {
        &mut self.encoder
    }
}
