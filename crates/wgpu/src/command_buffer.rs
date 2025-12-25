//! Define a command buffer to record commands for the GPU.
use bevy::{log::Level, log::tracing::event};
use wgpu::Texture;

use crate::{buffer::WBuffer, compute_pass::WComputePass, instance::WRenderInstanceData, texture::WTextureView};

use super::render_pass::WRenderPass;

/// Type of a color.
pub type WColor = wgpu::Color;

/// Type of a load operation.
pub type WLoadOp<V> = wgpu::LoadOp<V>;

/// Type of a store operation.
pub type WStoreOp = wgpu::StoreOp;

/// Load and store operations for the color texture.
#[derive(Clone, Copy, Debug)]
pub struct Operations<V> {
    pub load: WLoadOp<V>,
    pub store: WStoreOp,
}

/// Describe the depth texture of a render pass.
pub struct RenderPassDepth<'pass> {
    /// The depth texture. If `None`, the render pass will not have a depth texture.
    pub texture: Option<&'pass WTextureView>,
    /// The depth operation when loading the texture. By default, clear the texture to 1.0 (farthest).
    pub load_operation: WLoadOp<f32>,
    /// The depth operation when storing the texture. By default, store the texture.
    pub store_operation: WStoreOp,
}
impl Default for RenderPassDepth<'_> {
    fn default() -> Self {
        Self {
            texture: None,
            load_operation: wgpu::LoadOp::Clear(1.0),
            store_operation: wgpu::StoreOp::Store,
        }
    }
}

/// Describe a color attachment of a render pass.
pub struct RenderPassColorAttachment<'pass> {
    /// The color texture.
    pub texture: Option<&'pass WTextureView>,
    /// The color load operation. By default, clear the texture to a dark gray. The color must be in sRGB.
    pub load: WLoadOp<WColor>,
    /// The color store operation. By default, store the texture.
    pub store: WStoreOp,
}
impl Default for RenderPassColorAttachment<'_> {
    fn default() -> Self {
        let color_srgb = 0.1_f64.powf(2.2);
        Self {
            texture: None,
            load: wgpu::LoadOp::Clear(WColor { r: color_srgb, g: color_srgb, b: color_srgb, a: 1.0 }),
            store: wgpu::StoreOp::Store,
        }
    }
}

/// Builder for a render pass.
#[derive(Default)]
pub struct RenderPassBuilder<'pass> {
    /// The depth texture of the render pass.
    depth: RenderPassDepth<'pass>,
    /// The color attachments of the render pass. By default, no color attachments.
    color_attachments: Vec<RenderPassColorAttachment<'pass>>,
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


/// Create a new command buffer to record commands for the GPU.
/// The command buffer can be used to create render passes and compute passes.
/// Then, the command buffer can be submitted to the GPU.
/// 
/// # Example
/// 
/// ```
/// let mut command_buffer = WCommandBuffer::new(&instance, "Command Buffer");
/// 
/// {
///     // Create passes
///     let mut render_pass = command_buffer.create_render_pass(
///         "Render Pass", &color_texture,
///         Some(Operations { load: LoadOp::Clear(Color::BLACK), store: StoreOp::Store }),
///         Some(&depth_texture));
///     let mut compute_pass = command_buffer.create_compute_pass("Compute Pass");
/// 
///     // Use the render pass
///     // ...
/// }
/// 
/// // Submit the command buffer to the GPU
/// command_buffer.submit(&instance);
/// ```
pub struct WCommandBuffer {
    pub label: String,
    encoder: wgpu::CommandEncoder,
}

impl std::fmt::Debug for WCommandBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandBuffer")
            .field("label", &self.label)
            .finish()
    }
}

impl WCommandBuffer {
    /// Create a new command buffer.
    /// 
    /// # Arguments
    /// 
    /// * `instance` - The render instance.
    /// * `label` - The label of the command buffer.
    pub fn new(instance: &WRenderInstanceData<'_>, label: &str) -> Self {
        event!(Level::TRACE, "Creating a command buffer {}.", label);

        // Create command encoder
        let command_encoder = instance.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some(format!("{}-command-encoder", label).as_str()),
        });

        Self {
            label: label.to_string(),
            encoder: command_encoder,
        }
    }

    /// Create a new render pass.
    /// 
    /// # Arguments
    /// 
    /// * `label` - The label of the render pass.
    /// * `builder_func` - The function to build the render pass.
    pub fn create_render_pass<'pass>(
        &'pass mut self, label: &str, builder_func: impl FnOnce(&mut RenderPassBuilder<'pass>)
    ) -> WRenderPass<'pass> {
        event!(Level::TRACE, "Creating a render pass {}.", label);

        // Run the builder function
        let mut builder = RenderPassBuilder::default();
        builder_func(&mut builder);

        let mut depth_attachment = None;
        if let Some(depth_texture) = builder.depth.texture {
            depth_attachment = Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_texture,
                depth_ops: Some(wgpu::Operations {
                    load: builder.depth.load_operation,
                    store: builder.depth.store_operation
                }),
                stencil_ops: None,
            });
        }

        let color_attachments = builder.color_attachments.iter().map(|attachment| {
            attachment.texture.map(|texture| wgpu::RenderPassColorAttachment {
                view: texture,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: attachment.load,
                    store: attachment.store
                },
            })
        }).collect::<Vec<_>>();

        let render_pass = self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(format!("{}-render-pass", label).as_str()),
            color_attachments: &color_attachments,
            depth_stencil_attachment: depth_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        WRenderPass::new(label, render_pass)
    }

    /// Create a new compute pass.
    /// 
    /// # Arguments
    /// 
    /// * `label` - The label of the compute pass.
    pub fn create_compute_pass<'pass>(&'pass mut self, label: &str) -> WComputePass<'pass> {
        event!(Level::TRACE, "Creating a compute pass {}.", label);
        let compute_pass = self.encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
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
    pub fn submit(self, instance: &WRenderInstanceData) {
        event!(Level::TRACE, "Submitted command buffer {}.", self.label);
        instance.queue.submit(std::iter::once(self.encoder.finish()));
    }


    /// Copy a buffer to another buffer.
    /// Please use the `copy_from_buffer` method of the buffer to copy data.
    /// 
    /// # Arguments
    /// 
    /// * `source` - The source buffer.
    /// * `destination` - The destination buffer.
    pub fn copy_buffer_to_buffer(&mut self, source: &WBuffer, destination: &WBuffer) {
        event!(Level::TRACE, "Copying buffer {} to buffer {}.", source.label, destination.label);

        self.encoder.copy_buffer_to_buffer(
            &source.buffer, 0,
            &destination.buffer, 0,
            source.buffer.size());
    }

    /// Copy a texture to a buffer.
    /// Please use the `copy_from_texture` method of the buffer to copy data.
    /// 
    /// # Arguments
    /// 
    /// * `source` - The source texture.
    /// * `destination` - The destination buffer.
    /// * `size` - The size of the texture.
    pub fn copy_texture_to_buffer(&mut self, source: &Texture, destination: &WBuffer, size: wgpu::Extent3d) {
        event!(Level::TRACE, "Copying texture to buffer {}.", destination.label);

        // Create texture copy
        let texture_copy = source.as_image_copy();

        // Create buffer copy
        let buffer_copy = wgpu::ImageCopyBuffer {
            buffer: &destination.buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * 4 * size.width),
                rows_per_image: None,
            }
        };

        // Copy texture to buffer
        self.encoder.copy_texture_to_buffer(
            texture_copy,
            buffer_copy,
            size);
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
