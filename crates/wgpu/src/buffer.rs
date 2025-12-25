//! Contains the buffer struct and its implementations.

use std::fmt::Formatter;
use bevy::{log::Level, log::tracing::event};
use wgpu::{util::DeviceExt, BufferView};

use crate::{command_buffer::WCommandBuffer, instance::WRenderInstanceData};

/// Buffer usages.
pub type BufferUsage = wgpu::BufferUsages;

/// Buffer binding types.
pub type BufferBindingType = wgpu::BufferBindingType;

/// Map the buffer.
pub type BufferViewMut<'a> = wgpu::BufferViewMut<'a>;

/// Create a buffer on the GPU.
/// 
/// # Example
/// 
/// ```
/// // Create a new buffer
/// let mut buffer = WBuffer::new(&instance, "Buffer label", 1024, BufferUsage::Vertex, None);
/// 
/// // Copy data to the buffer
/// buffer.copy_from_buffer(&instance, &buffer);
/// 
/// // Copy data to the buffer from a texture
/// buffer.copy_from_texture(&instance, &texture);
/// 
/// // Write data to the buffer starting at 16 bytes
/// buffer.write(&instance, bytemuck::cast_slice(&[data]), 16);
/// 
/// // Map the buffer and read the data
/// buffer.map_read(&instance, |data| {
///   let data = bytemuck::cast_slice(data);
///   // ...
/// });
/// 
/// // Map the buffer and write the data
/// buffer.map_write(&instance, |data| {
///   let data = bytemuck::cast_slice_mut(data);
///   // ...
/// });
/// ```
pub struct WBuffer {
    pub label: String,
    pub buffer: wgpu::Buffer,
}

impl std::fmt::Debug for WBuffer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Buffer")
            .field("label", &self.label)
            .field("buffer_size", &self.buffer.size())
            .finish()
    }
}

impl WBuffer {
    /// Create a new buffer.
    /// 
    /// # Arguments
    /// 
    /// * `instance` - The render instance.
    /// * `label` - The label of the buffer.
    /// * `size` - The size of the buffer.
    /// * `usage` - The usage of the buffer (vertex, index, uniform, storage).
    /// * `content` - The content of the buffer.
    pub fn new(instance: &WRenderInstanceData, label: &str, size: usize, usage: BufferUsage, content: Option<&[u8]>) -> Self {
        event!(Level::TRACE, "Creating new buffer {}.", label);

        // In case the content is not provided, create an empty buffer.
        match content {
            Some(content) => {
                // Create buffer
                let buffer = instance.device.create_buffer_init(
                    &wgpu::util::BufferInitDescriptor {
                        label: Some(format!("{}-buffer", label).as_str()),
                        contents: content,
                        usage 
                    }
                );
                
                WBuffer {
                    label: label.to_string(),
                    buffer,
                }
            },
            None => {
                // Create empty buffer of the given size
                let buffer = instance.device.create_buffer(
                    &wgpu::BufferDescriptor {
                        label: Some(format!("{}-buffer", label).as_str()),
                        size: size as u64,
                        usage,
                        mapped_at_creation: false,
                    }
                );

                WBuffer {
                    label: label.to_string(),
                    buffer,
                }
            },
        }
    }


    /// Copy data to the buffer from another buffer.
    /// Note that the buffer must have the COPY_DST usage.
    /// 
    /// # Arguments
    /// 
    /// * `instance` - The render instance.
    /// * `buffer` - The buffer to copy from.
    pub fn copy_from_buffer(&self, instance: &WRenderInstanceData<'_>, buffer: &WBuffer) {
        event!(Level::TRACE, "Copying data from buffer {} to buffer {}.", buffer.label, self.label);

        // Create command encoder
        let mut command_buffer = WCommandBuffer::new(
            instance,
            &format!("copy-from-{}-to-{}", buffer.label, self.label));

        // Copy buffer
        command_buffer.copy_buffer_to_buffer(buffer, self);

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
    pub fn copy_from_texture(&mut self, instance: &WRenderInstanceData<'_>, texture: &wgpu::Texture) {
        event!(Level::TRACE, "Copying data from texture to buffer {}.", self.label);

        // Create command encoder
        let mut command_buffer = WCommandBuffer::new(
            instance,
            &format!("copy-to-{}", self.label));

        // Copy texture
        command_buffer.copy_texture_to_buffer(texture, self, texture.size());

        // Submit commands
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
    pub fn write(&mut self, instance: &WRenderInstanceData, content: &[u8], offset: usize) {
        event!(Level::TRACE, "Writing data to buffer {}.", self.label);

        instance.queue.write_buffer(
            &self.buffer,
            offset as u64,
            content);
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
    ///     The callback takes reference to the buffer.
    pub fn map_read(&self, instance: &WRenderInstanceData, callback: impl FnOnce(BufferView)) {
        event!(Level::TRACE, "Mapping buffer {} for reading.", self.label);

        // Map buffer
        let (sender, receiver) = std::sync::mpsc::channel();
        let buffer_slice = self.buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read,
            move |r| sender.send(r).unwrap());

        // Wait for the buffer to be mapped
        instance.device.poll(wgpu::Maintain::Wait);
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
    ///     The callback takes a mutable reference to the buffer.
    pub fn map_write(&self, instance: &WRenderInstanceData, callback: impl FnOnce(BufferViewMut)) {
        event!(Level::TRACE, "Mapping buffer {} for writing.", self.label);

        // Map buffer
        let (sender, receiver) = std::sync::mpsc::channel();
        let buffer_slice = self.buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Write,
            move |r| sender.send(r).unwrap());

        // Wait for the buffer to be mapped
        instance.device.poll(wgpu::Maintain::Wait);
        receiver.recv().unwrap().unwrap();

        // Call callback
        callback(buffer_slice.get_mapped_range_mut());

        // Unmap buffer
        self.buffer.unmap();
    }
}
