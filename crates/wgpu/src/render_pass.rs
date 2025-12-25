//! Render pass abstraction for the WGPU library.

use std::ops::Range;

use bevy::log::error;
use bevy::log::Level;
use bevy::log::tracing::event;
use wgpu::BufferAddress;
use wgpu::ShaderStages;

use crate::buffer::WBuffer;
use crate::instance::WRenderError;

use super::render_pipeline::WRenderPipeline;

// Alias struct for the draw indirect functions.
pub use wgpu::util::DrawIndirectArgs;
pub use wgpu::util::DrawIndexedIndirectArgs;

/// Create a render pass instance.
/// 
/// # Example
/// 
/// To set the render pass dependencies and render primitives:
/// ```rust
/// let mut render_pass = WRenderPass::new("Render pass name");
/// 
/// // Set render pass dependencies
/// render_pass
///     .set_scissor_rect(0, 0, 800, 600) // Set the scissor rect to (0, 0) with width 800 and height 600
///     .set_vertex_buffer(vertex_buffer) // Set the vertex buffer of the current render pass
///     .set_index_buffer(index_buffer);  // Set the index buffer of the current render pass
/// 
/// render_pass
///     .set_pipeline(&render_pipeline)  // Set the pipeline of the render pass. The pipeline must be initialized.
///     .set_push_constants(ShaderType::Vertex, bytemuck::cast_slice(&[...])) // Set push constants values
///     .set_bind_group(0, &bind_group); // Set bind group at binding 0
/// ```
/// 
/// You can then render primitives using the different methods of the render pass.
/// The following methods are available:
/// 
/// ```rust
/// // Render primitives
/// // The first parameter is the range of vertices to draw, and the second parameter is the range of instances to draw.
/// render_pass.draw(first_vertex..last_vertex, first_instance_index..last_instance_index);
/// 
/// // Render indexed
/// // The first parameter is the range of indices to draw, and the second parameter is the range of instances to draw.
/// render_pass.draw_indexed(first_index..last_index, first_instance_index..last_instance_index);
/// 
/// // Draw primitives from the active vertex buffers.
/// // The draw is indirect, meaning the draw arguments are read from a buffer.
/// // The first parameter is the offset (the starting command in the buffer), and the second parameter is the number of commands to execute.
/// render_pass.multi_draw_indirect(indirect_buffer, first_draw_command_index, draw_commands_count);
/// 
/// // Draw primitives from the active vertex buffers as indexed triangles.
/// // The draw is indirect, meaning the draw arguments are read from a buffer.
/// // The first parameter is the offset (the starting command in the buffer), and the second parameter is the number of commands to execute.
/// render_pass.multi_draw_indexed_indirect(indirect_buffer, first_draw_command_index, draw_commands_count);
/// ```
pub struct WRenderPass<'a> {
    pub label: String,
    render_pass: wgpu::RenderPass<'a>,
    pipeline_set: bool,
    vertex_buffer_set: bool,
    index_buffer_set: bool,
}

impl std::fmt::Debug for WRenderPass<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderPass")
            .field("label", &self.label)
            .finish()
    }
}

impl<'a> WRenderPass<'a> {
    /// Create a new render pass.
    /// 
    /// # Arguments
    /// 
    /// * `label` - The label of the render pass.
    /// * `render_pass` - The render pass to create.
    pub fn new(label: &str, render_pass: wgpu::RenderPass<'a>) -> Self {
        event!(Level::TRACE, "Creating a new render pass {}.", label);

        Self {
            label: label.to_string(),
            render_pass,
            pipeline_set: false,
            vertex_buffer_set: false,
            index_buffer_set: false,
        }
    }

    /// Set the pipeline of the render pass.
    /// The bind groups of the pipeline are also set.
    /// 
    /// # Arguments
    /// 
    /// * `pipeline` - The pipeline to set.
    /// 
    /// # Errors
    /// 
    /// * `RenderError::PipelineNotInitialized` - The pipeline is not initialized.
    pub fn set_pipeline(&mut self, pipeline: &'a WRenderPipeline) -> Result<&mut Self, WRenderError> {
        if pipeline.get_pipeline().is_none() {
            error!(pipeline.label, "Pipeline is not created yet.");
            return Err(WRenderError::PipelineNotInitialized);
        }

        // Set pipeline
        self.render_pass.set_pipeline(pipeline.get_pipeline().as_ref().unwrap());
        self.pipeline_set = true;
        Ok(self)
    }

    /// Set a vertex buffer of the render pass.
    /// 
    /// # Arguments
    /// 
    /// * `binding` - The binding of the vertex buffer.
    /// * `buffer` - The buffer to set.
    pub fn set_vertex_buffer(&mut self, binding: u32, buffer: &'a WBuffer) -> &mut Self {
        self.render_pass.set_vertex_buffer(binding, buffer.buffer.slice(..));
        self.vertex_buffer_set = true;
        self
    }

    /// Set the index buffer of the render pass.
    /// 
    /// # Arguments
    /// 
    /// * `buffer` - The buffer to set.
    pub fn set_index_buffer(&mut self, buffer: &'a WBuffer) -> &mut Self {
        self.render_pass.set_index_buffer(buffer.buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.index_buffer_set = true;
        self
    }

    /// Set the scissor rect of the render pass.
    /// 
    /// # Arguments
    /// 
    /// * `x` - X coordinate of the scissor rect.
    /// * `y` - Y coordinate of the scissor rect.
    /// * `width` - Width of the scissor rect.
    /// * `height` - Height of the scissor rect.
    pub fn set_scissor_rect(&mut self, x: u32, y: u32, width: u32, height: u32) -> &mut Self {
        self.render_pass.set_scissor_rect(x, y, width, height);
        self
    }



    /// Set push constants of the render pass.
    /// 
    /// # Arguments
    /// 
    /// * `stages` - The shader stages to set the push constants for.
    /// * `data` - The data to set.
    pub fn set_push_constants(&mut self, stages: ShaderStages, data: &[u8]) -> &mut Self {
        self.render_pass.set_push_constants(stages, 0, data);
        self
    }

    /// Set a bind group of the render pass at a binding.
    /// 
    /// # Arguments
    /// 
    /// * `binding` - The binding of the bind group.
    /// * `bind_group` - The bind group to set.
    pub fn set_bind_group(&mut self, binding: u32, bind_group: &'a wgpu::BindGroup) -> &mut Self {
        self.render_pass.set_bind_group(binding, bind_group, &[]);
        self
    }



    /// Draws primitives from the active vertex buffers.
    /// 
    /// # Arguments
    /// 
    /// * `vertices` - Range of vertices to draw.
    /// * `instances` - Range of instances to draw.
    /// 
    /// # Errors
    /// 
    /// * `RenderError::PipelineNotSet` - The pipeline is not set.
    /// * `RenderError::MissingVertexBuffer` - The vertex buffer is not set.
    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) -> Result<(), WRenderError> {
        if !self.pipeline_set {
            error!(self.label, "Pipeline is not set.");
            return Err(WRenderError::PipelineNotSet);
        }
        if !self.vertex_buffer_set {
            error!(self.label, "Vertex buffer is not set.");
            return Err(WRenderError::MissingVertexBuffer);
        }
        event!(Level::TRACE, "Drawing {} vertices and {} instances.", vertices.end - vertices.start, instances.end - instances.start);
        self.render_pass.draw(vertices, instances);
        Ok(())
    }

    /// Draws primitives from the active vertex buffers as indexed triangles.
    /// 
    /// # Arguments
    /// 
    /// * `indices` - Range of indices to draw.
    /// * `instance_index` - Indice of the instance to draw.
    /// 
    /// # Errors
    /// 
    /// * `RenderError::PipelineNotSet` - The pipeline is not set.
    /// * `RenderError::MissingVertexBuffer` - The vertex buffer is not set.
    /// * `RenderError::MissingIndexBuffer` - The index buffer is not set.
    pub fn draw_indexed(&mut self, indices: Range<u32>, instance_index: Range<u32>) -> Result<(), WRenderError> {
        if !self.pipeline_set {
            error!(self.label, "Pipeline is not set.");
            return Err(WRenderError::PipelineNotSet);
        }
        if !self.vertex_buffer_set {
            error!(self.label, "Vertex buffer is not set.");
            return Err(WRenderError::MissingVertexBuffer);
        }
        if !self.index_buffer_set {
            error!(self.label, "Index buffer is not set.");
            return Err(WRenderError::MissingIndexBuffer);
        }
        event!(Level::TRACE, "Drawing indexed {} indices and {} instances.", indices.end - indices.start, instance_index.end - instance_index.start);
        self.render_pass.draw_indexed(indices, 0, instance_index);
        Ok(())
    }



    /// Draws primitives from the active vertex buffers.
    /// The draw is indirect, meaning the draw arguments are read from a buffer.
    /// 
    /// # Arguments
    /// 
    /// * `buffer` - The buffer to read the draw arguments from.
    /// * `offset` - The first draw argument to read.
    /// * `count` - The number of draw arguments to read.
    /// 
    /// # Errors
    /// 
    /// * `RenderError::PipelineNotSet` - The pipeline is not set.
    /// * `RenderError::MissingVertexBuffer` - The vertex buffer is not set.
    pub fn multi_draw_indirect(&mut self, buffer: &'a WBuffer, offset: BufferAddress, count: u32) -> Result<(), WRenderError> {
        if !self.pipeline_set {
            error!(self.label, "Pipeline is not set.");
            return Err(WRenderError::PipelineNotSet);
        }
        if !self.vertex_buffer_set {
            error!(self.label, "Vertex buffer is not set.");
            return Err(WRenderError::MissingVertexBuffer);
        }
        event!(Level::TRACE, "Drawing {} instances from indirect buffer.", count);
        self.render_pass.multi_draw_indirect(&buffer.buffer, offset, count);
        Ok(())
    }

    /// Draws primitives from the active vertex buffers as indexed triangles.
    /// The draw is indirect, meaning the draw arguments are read from a buffer.
    /// 
    /// # Arguments
    /// 
    /// * `buffer` - The buffer to read the draw arguments from.
    /// * `offset` - The first draw argument to read.
    /// * `count` - The number of draw arguments to read.
    /// 
    /// # Errors
    /// 
    /// * `RenderError::PipelineNotSet` - The pipeline is not set.
    /// * `RenderError::MissingVertexBuffer` - The vertex buffer is not set.
    /// * `RenderError::MissingIndexBuffer` - The index buffer is not set.
    pub fn multi_draw_indexed_indirect(&mut self, buffer: &'a WBuffer, offset: BufferAddress, count: u32) -> Result<(), WRenderError> {
        if !self.pipeline_set {
            error!(self.label, "Pipeline is not set.");
            return Err(WRenderError::PipelineNotSet);
        }
        if !self.vertex_buffer_set {
            error!(self.label, "Vertex buffer is not set.");
            return Err(WRenderError::MissingVertexBuffer);
        }
        if !self.index_buffer_set {
            error!(self.label, "Index buffer is not set.");
            return Err(WRenderError::MissingIndexBuffer);
        }
        event!(Level::TRACE, "Drawing indexed {} instances from indirect buffer.", count);
        self.render_pass.multi_draw_indexed_indirect(&buffer.buffer, offset, count);
        Ok(())
    }
}
