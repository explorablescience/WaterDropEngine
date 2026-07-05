//! GPU buffer helper utilities built on top of `wgpu::Buffer`.

use std::{fmt::Formatter, sync::mpsc};
use wde_logger::prelude::*;
use wgpu::{BufferView, util::DeviceExt};

use crate::{command_buffer::CommandBuffer, instance::RenderInstanceData};

/// Buffer usages.
pub type BufferUsage = wgpu::BufferUsages;

/// Buffer binding types.
pub type BufferBindingType = wgpu::BufferBindingType;

/// Map the buffer.
pub type BufferViewMut = wgpu::BufferViewMut;

/// Create and manage GPU buffers.
///
/// # Examples
/// Write once, render many:
/// ```rust,no_run
/// use wde_wgpu::{buffer::{Buffer, BufferUsage}, instance::RenderInstanceData};
///
/// let vertices: &[f32] = &[0.0, 0.5, 0.0, -0.5, -0.5, 0.0, 0.5, -0.5, 0.0];
/// let mut vertex_buffer = Buffer::new(
///     instance,
///     "triangle-vertices",
///     vertices.len() * std::mem::size_of::<f32>(),
///     BufferUsage::VERTEX | BufferUsage::COPY_DST,
///     None,
/// );
/// vertex_buffer.write(instance, bytemuck::cast_slice(vertices), 0);
/// ```
///
/// Read back GPU results:
/// ```rust,no_run
/// use wde_wgpu::{buffer::{Buffer, BufferUsage}, instance::RenderInstanceData};
///
/// let readback = Buffer::new(
///     instance,
///     "readback",
///     1024,
///     BufferUsage::MAP_READ | BufferUsage::COPY_DST,
///     None,
/// );
/// readback.map_read(instance, |view| {
///     let floats: &[f32] = bytemuck::cast_slice(&view);
///     println!("first value = {}", floats.get(0).unwrap_or(&0.0));
/// });
/// ```
///
/// Copy between GPU resources:
/// ```rust,no_run
/// use wde_wgpu::{buffer::{Buffer, BufferUsage}, instance::RenderInstanceData};
///
/// let src = Buffer::new(instance, "src", 256, BufferUsage::COPY_SRC, Some(&[1u8; 256]));
/// let dst = Buffer::new(instance, "dst", 256, BufferUsage::COPY_DST, None);
/// dst.copy_from_buffer(instance, &src);
/// ```
pub struct Buffer {
    pub label: String,
    pub buffer: wgpu::Buffer
}

impl std::fmt::Debug for Buffer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Buffer")
            .field("label", &self.label)
            .field("buffer_size", &self.buffer.size())
            .finish()
    }
}

impl Buffer {
    /// Create a new buffer.
    ///
    /// # Arguments
    ///
    /// * `instance` - The render instance.
    /// * `label` - The label of the buffer.
    /// * `size` - The size of the buffer.
    /// * `usage` - The usage of the buffer (vertex, index, uniform, storage).
    /// * `content` - The content of the buffer.
    pub fn new(
        instance: &RenderInstanceData,
        label: &str,
        size: usize,
        usage: BufferUsage,
        content: Option<&[u8]>
    ) -> Self {
        event!(LogLevel::TRACE, "Creating new buffer {}.", label);

        // In case the content is not provided, create an empty buffer.
        match content {
            Some(content) => {
                // Create buffer
                let buffer =
                    instance
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(format!("{}-buffer", label).as_str()),
                            contents: content,
                            usage
                        });

                Buffer {
                    label: label.to_string(),
                    buffer
                }
            }
            None => {
                // Create empty buffer of the given size
                let buffer = instance.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(format!("{}-buffer", label).as_str()),
                    size: size as u64,
                    usage,
                    mapped_at_creation: false
                });

                Buffer {
                    label: label.to_string(),
                    buffer
                }
            }
        }
    }

    /// Copy data to the buffer from another buffer.
    /// Note that the buffer must have the COPY_DST usage.
    ///
    /// # Arguments
    ///
    /// * `instance` - The render instance.
    /// * `buffer` - The buffer to copy from.
    pub fn copy_from_buffer(&self, instance: &RenderInstanceData<'_>, buffer: &Buffer) {
        // Create command encoder
        let mut command_buffer = CommandBuffer::new(
            instance,
            &format!("copy-from-{}-to-{}", buffer.label, self.label)
        );

        // Copy buffer
        command_buffer.copy_buffer_to_buffer(buffer, self);

        // Submit commands
        command_buffer.submit(instance);
    }

    /// Copy data to the buffer from another buffer offset by a given offset.
    /// Note that the buffer must have the COPY_DST usage.
    ///
    /// # Arguments
    ///
    /// * `instance` - The render instance.
    /// * `buffer` - The buffer to copy from.
    /// * `source_offset` - The offset in the source buffer to start copying from.
    /// * `destination_offset` - The offset in the destination buffer to start copying to.
    /// * `size` - The size of the data to copy.
    pub fn copy_from_buffer_offset(
        &self,
        instance: &RenderInstanceData<'_>,
        buffer: &Buffer,
        source_offset: u64,
        destination_offset: u64,
        size: u64
    ) {
        // Create command encoder
        let mut command_buffer = CommandBuffer::new(
            instance,
            &format!("copy-from-{}-to-{}", buffer.label, self.label)
        );

        // Copy buffer
        command_buffer.copy_buffer_to_buffer_offset(
            buffer,
            self,
            source_offset,
            destination_offset,
            size
        );

        // Submit commands
        command_buffer.submit(instance);
    }

    /// Copy data to the buffer from a texture.
    /// Note that the buffer must have the COPY_DST usage.
    ///
    /// # Arguments
    ///
    /// * `instance` - The render instance.
    /// * `texture` - The texture to copy from.
    pub fn copy_from_texture(
        &mut self,
        instance: &RenderInstanceData<'_>,
        texture: &wgpu::Texture
    ) {
        let mut command_buffer = CommandBuffer::new(instance, &format!("copy-to-{}", self.label));
        command_buffer.copy_texture_to_buffer(texture, self, texture.size());
        command_buffer.submit(instance);
    }

    /// Copy a single layer from a texture array into this buffer.
    pub fn copy_from_texture_layered(
        &mut self,
        instance: &RenderInstanceData<'_>,
        texture: &wgpu::Texture,
        layer: u32
    ) {
        let mut command_buffer = CommandBuffer::new(instance, &format!("copy-to-{}", self.label));
        command_buffer.copy_texture_layer_to_buffer(texture, self, layer);
        command_buffer.submit(instance);
    }

    /// Write data to the buffer.
    /// Note that the buffer must have the COPY_DST usage.
    ///
    /// # Arguments
    ///
    /// * `instance` - The render instance.
    /// * `content` - The content to write to the buffer.
    /// * `offset` - The offset to write the content to.
    pub fn write(&self, instance: &RenderInstanceData, content: &[u8], offset: usize) {
        instance
            .queue
            .write_buffer(&self.buffer, offset as u64, content);
    }

    /// Map the buffer.
    /// The access to the buffer is read-only.
    /// This will wait for the buffer to be mapped.
    /// Note that the buffer must have the MAP_READ usage.
    ///
    /// # Arguments
    ///
    /// * `instance` - The render instance.
    /// * `callback` - A closure that takes a reference to the buffer.
    ///   The callback takes reference to the buffer.
    pub fn map_read(&self, instance: &RenderInstanceData, callback: impl FnOnce(BufferView)) {
        // Map buffer
        let (sender, receiver) = std::sync::mpsc::channel();
        let buffer_slice = self.buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());

        // Wait for the buffer to be mapped
        if let Err(e) = instance.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None
        }) {
            error!(
                "Failed to poll device while mapping buffer '{}': {:?}",
                self.label, e
            );
        }
        receiver.recv().unwrap().unwrap();

        // Call callback
        callback(buffer_slice.get_mapped_range());

        // Unmap buffer
        self.buffer.unmap();
    }

    /// Map the buffer as mutable.
    /// This allows to write to the buffer.
    /// This will wait for the buffer to be mapped.
    /// Note that the buffer must have the MAP_WRITE usage.
    ///
    /// # Arguments
    ///
    /// * `instance` - The render instance.
    /// * `callback` - A closure that takes a mutable reference to the buffer.
    ///   The callback takes a mutable reference to the buffer.
    pub fn map_write(&self, instance: &RenderInstanceData, callback: impl FnOnce(BufferViewMut)) {
        // Map buffer
        let (sender, receiver) = std::sync::mpsc::channel();
        let buffer_slice = self.buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Write, move |r| sender.send(r).unwrap());

        // Wait for the buffer to be mapped
        {
            let _span = trace_span!("wait_for_buffer_mapping").entered();
            if let Err(e) = instance.device.poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None
            }) {
                error!(
                    "Failed to poll device while mapping buffer '{}': {:?}",
                    self.label, e
                );
            }
            receiver.recv().unwrap().unwrap();
        }

        // Call callback
        callback(buffer_slice.get_mapped_range_mut());

        // Unmap buffer
        self.buffer.unmap();
    }
}

/// A pending GPU→CPU readback. Created by [`AsyncReadback::from_texture_layer`].
/// Call [`AsyncReadback::try_collect`] each frame (after a non-blocking device poll)
/// until it returns `Ok(data)`.
pub struct AsyncReadback {
    buffer: wgpu::Buffer,
    receiver: mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>
}

// wgpu::Buffer is Send+Sync so AsyncReadback can cross thread boundaries.
unsafe impl Send for AsyncReadback {}
unsafe impl Sync for AsyncReadback {}

impl AsyncReadback {
    /// Copy one layer of a texture array into a fresh staging buffer and start an
    /// async mapping. The GPU copy and the mapping are both non-blocking — the caller
    /// must poll the device (e.g. `PollType::Poll`) and call `try_collect` in a later frame.
    pub fn from_texture_layer(
        instance: &RenderInstanceData,
        texture: &wgpu::Texture,
        layer: u32
    ) -> Self {
        let size = texture.size();
        let bytes_per_pixel: u32 = match texture.format() {
            wgpu::TextureFormat::R8Unorm => 1,
            wgpu::TextureFormat::Rgba8Unorm => 4,
            fmt => panic!("Unsupported format for async readback: {:?}", fmt)
        };
        // wgpu requires bytes_per_row to be a multiple of 256.
        let bytes_per_row = (size.width * bytes_per_pixel).div_ceil(256) * 256;
        let buffer_size = (bytes_per_row * size.height) as u64;

        let staging = instance.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("async-readback-staging"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false
        });

        let mut encoder = instance
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("async-readback-copy")
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: layer },
                aspect: wgpu::TextureAspect::All
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: None
                }
            },
            wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1
            }
        );
        instance.queue.submit(std::iter::once(encoder.finish()));

        let (sender, receiver) = mpsc::channel();
        staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| { let _ = sender.send(r); });

        AsyncReadback { buffer: staging, receiver }
    }

    /// Try to collect the result. Returns `Ok(data)` when the GPU has finished, or
    /// `Err(self)` when the mapping is still in progress (call again next frame).
    /// Call `instance.device.poll(PollType::Poll)` before this to process GPU callbacks.
    pub fn try_collect(self) -> Result<Vec<u8>, Self> {
        match self.receiver.try_recv() {
            Ok(Ok(())) => {
                let data = self.buffer.slice(..).get_mapped_range().to_vec();
                self.buffer.unmap();
                Ok(data)
            }
            Ok(Err(e)) => {
                error!("Async readback mapping error: {:?}", e);
                Ok(Vec::new())
            }
            Err(mpsc::TryRecvError::Empty) => Err(self),
            Err(mpsc::TryRecvError::Disconnected) => Ok(Vec::new())
        }
    }
}
