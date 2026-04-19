//! Render pass abstraction for the WGPU library.

use std::ops::Range;

use wde_logger::prelude::*;
use wgpu::BufferAddress;
use wgpu::ShaderStages;

use crate::buffer::Buffer;
use crate::instance::RenderError;
use crate::pipelines::BindGroup;
use crate::render_pipeline::RenderPipeline;

// Alias struct for the draw indirect functions.
pub use wgpu::util::DrawIndexedIndirectArgs;
pub use wgpu::util::DrawIndirectArgs;

/// RAII wrapper over `wgpu::RenderPass` with guard rails to prevent draws without required
/// state (pipeline + vertex/index buffers).
///
/// # Example
/// ```rust,no_run
/// use wde_wgpu::{
///     buffer::Buffer,
///     render_pass::RenderPass,
///     render_pipeline::RenderPipeline,
/// };
///
/// let mut pass = RenderPassInstance::new("example", render_pass);
/// pass
///     .set_pipeline(pipeline).unwrap()
///     .set_vertex_buffer(0, vbo)
///     .set_index_buffer(ibo)
///     .draw_indexed(0..36, 0..1)
///     .unwrap();
/// ```
///
/// Supported operations include standard draws, indexed draws, and indirect versions of
/// both. Errors are returned if mandatory state (pipeline, vertex/index buffers) is missing.
pub struct RenderPassInstance<'a> {
    pub label: String,
    render_pass: wgpu::RenderPass<'a>,
    pipeline_set: bool,
    vertex_buffer_set: bool,
    index_buffer_set: bool
}

impl std::fmt::Debug for RenderPassInstance<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderPassInstance")
            .field("label", &self.label)
            .finish()
    }
}

impl<'a> RenderPassInstance<'a> {
    /// Create a new render pass.
    ///
    /// # Arguments
    ///
    /// * `label` - The label of the render pass.
    /// * `render_pass` - The render pass to create.
    pub fn new(label: &str, render_pass: wgpu::RenderPass<'a>) -> Self {
        Self {
            label: label.to_string(),
            render_pass,
            pipeline_set: false,
            vertex_buffer_set: false,
            index_buffer_set: false
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
    pub fn set_pipeline(&mut self, pipeline: &'a RenderPipeline) -> Result<&mut Self, RenderError> {
        if pipeline.get_pipeline().is_none() {
            error!(pipeline.label, "Pipeline is not created yet.");
            return Err(RenderError::PipelineNotInitialized);
        }

        // Set pipeline
        self.render_pass
            .set_pipeline(pipeline.get_pipeline().as_ref().unwrap());
        self.pipeline_set = true;
        Ok(self)
    }

    /// Set a vertex buffer of the render pass.
    ///
    /// # Arguments
    ///
    /// * `binding` - The binding of the vertex buffer.
    /// * `buffer` - The buffer to set.
    pub fn set_vertex_buffer(&mut self, binding: u32, buffer: &'a Buffer) -> &mut Self {
        self.render_pass
            .set_vertex_buffer(binding, buffer.buffer.slice(..));
        self.vertex_buffer_set = true;
        self
    }

    /// Set the index buffer of the render pass.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The buffer to set.
    pub fn set_index_buffer(&mut self, buffer: &'a Buffer) -> &mut Self {
        self.render_pass
            .set_index_buffer(buffer.buffer.slice(..), wgpu::IndexFormat::Uint32);
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
    pub fn set_bind_group(&mut self, binding: u32, bind_group: &'a BindGroup) -> &mut Self {
        debug_assert!(
            bind_group.0.is_some(),
            "Bind group {} is not created yet.",
            binding
        );
        self.render_pass
            .set_bind_group(binding, bind_group.0.as_ref().unwrap(), &[]);
        self
    }

    /// Set the stencil reference value used by stencil compare and write operations.
    pub fn set_stencil_reference(&mut self, reference: u32) -> &mut Self {
        self.render_pass.set_stencil_reference(reference);
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
    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) -> Result<(), RenderError> {
        if !self.pipeline_set {
            error!(self.label, "Pipeline is not set.");
            return Err(RenderError::PipelineNotSet);
        }
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
    pub fn draw_indexed(
        &mut self,
        indices: Range<u32>,
        instance_index: Range<u32>
    ) -> Result<(), RenderError> {
        if !self.pipeline_set {
            error!(self.label, "Pipeline is not set.");
            return Err(RenderError::PipelineNotSet);
        }
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
    pub fn multi_draw_indirect(
        &mut self,
        buffer: &'a Buffer,
        offset: BufferAddress,
        count: u32
    ) -> Result<(), RenderError> {
        if !self.pipeline_set {
            error!(self.label, "Pipeline is not set.");
            return Err(RenderError::PipelineNotSet);
        }
        if !self.vertex_buffer_set {
            error!(self.label, "Vertex buffer is not set.");
            return Err(RenderError::MissingVertexBuffer);
        }
        event!(
            LogLevel::TRACE,
            "Drawing {} instances from indirect buffer.",
            count
        );
        self.render_pass
            .multi_draw_indirect(&buffer.buffer, offset, count);
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
    pub fn multi_draw_indexed_indirect(
        &mut self,
        buffer: &'a Buffer,
        offset: BufferAddress,
        count: u32
    ) -> Result<(), RenderError> {
        if !self.pipeline_set {
            error!(self.label, "Pipeline is not set.");
            return Err(RenderError::PipelineNotSet);
        }
        self.render_pass
            .multi_draw_indexed_indirect(&buffer.buffer, offset, count);
        Ok(())
    }

    /// Forgets the lifetime of the render pass.
    /// This is used to return the inner `wgpu::RenderPass` when needed.
    pub fn forget_lifetime(self) -> RenderPassInstance<'static> {
        let render_pass = self.render_pass.forget_lifetime();
        RenderPassInstance {
            label: self.label,
            render_pass,
            pipeline_set: self.pipeline_set,
            vertex_buffer_set: self.vertex_buffer_set,
            index_buffer_set: self.index_buffer_set
        }
    }

    /// Consumes the render pass and returns the inner `wgpu::RenderPass`.
    pub fn into_inner(self) -> wgpu::RenderPass<'a> {
        self.render_pass
    }
}
