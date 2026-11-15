//! Render pipeline module.

use wde_logger::prelude::*;
use wgpu::BindGroupLayout;

use super::shader_module::{create_shader_module, validate_pipeline};
use crate::{
    instance::{RenderError, RenderInstanceData},
    texture::{DEPTH_FORMAT, SWAPCHAIN_FORMAT, TextureFormat},
    vertex::Vertex
};

/// List of available shaders.
pub type ShaderStages = wgpu::ShaderStages;
/// Type of the shader module.
pub type ShaderModule = wgpu::ShaderModule;
/// Export culling params.
pub type Face = wgpu::Face;
/// Export compare function.
pub type CompareFunction = wgpu::CompareFunction;

/// Depth
pub type StencilState = wgpu::StencilState;
pub type StencilFaceState = wgpu::StencilFaceState;
pub type StencilOperation = wgpu::StencilOperation;

/// Blend state
pub type BlendState = wgpu::BlendState;
pub type BlendComponent = wgpu::BlendComponent;
pub type BlendFactor = wgpu::BlendFactor;
pub type BlendOperation = wgpu::BlendOperation;
pub type ColorWrites = wgpu::ColorWrites;

/// Describes an optional depth attachment for a pipeline.
#[derive(Clone)]
pub struct DepthDescriptor {
    /// Whether the pipeline should have a depth attachment.
    pub enabled: bool,
    /// Whether the stencil attachment should be read-only.
    pub write: bool,
    /// The comparison function that the depth attachment will use.
    pub compare: CompareFunction,
    /// The stencil state for the depth attachment. If `None`, stencil testing is disabled.
    pub stencil: StencilState,
    /// Override the depth texture format. Defaults to `DEPTH_FORMAT` when `None`.
    pub format: Option<TextureFormat>
}
impl Default for DepthDescriptor {
    fn default() -> Self {
        Self {
            enabled: false,
            write: true,
            compare: CompareFunction::Less,
            stencil: StencilState::default(),
            format: None
        }
    }
}

/// Convenience enum that maps to `wgpu::PrimitiveTopology`.
#[derive(Clone, Copy)]
pub enum RenderTopology {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip
}

// Render pipeline configuration
struct RenderPipelineConfig {
    depth: DepthDescriptor,
    render_targets: Vec<TextureFormat>,
    primitive_topology: wgpu::PrimitiveTopology,
    push_constants: Vec<wgpu::PushConstantRange>,
    bind_groups: Vec<wgpu::BindGroupLayout>,
    vertex_shader: String,
    fragment_shader: String,
    fragment_blend: Option<BlendState>,
    color_write: ColorWrites,
    cull_mode: Option<Face>,
    sample_count: u32,
    use_vertices_buffer: bool
}

/// Stores a render pipeline.
///
/// # Example
/// ```rust,no_run
/// use wde_wgpu::render_pipeline::{DepthStencilDescriptor, RenderPipeline, RenderTopology, ShaderStages};
///
/// let mut pipeline = RenderPipeline::new("gbuffer");
/// pipeline
///     .set_shader(include_str!("../../../res/pbr/gbuffer.vert.wgsl"), ShaderStages::VERTEX)
///     .set_shader(include_str!("../../../res/pbr/gbuffer.frag.wgsl"), ShaderStages::FRAGMENT)
///     .set_topology(RenderTopology::TriangleList)
///     .set_depth(DepthStencilDescriptor { enabled: true, write: true, compare: wgpu::CompareFunction::Less })
///     .set_bind_groups(layouts)
///     .add_push_constant(ShaderStages::VERTEX, 0, 64)
///     .init(instance)
///     .expect("shaders validated");
/// assert!(pipeline.is_initialized());
///
/// let _wgpu_pipeline = pipeline.get_pipeline().unwrap();
/// ```
pub struct RenderPipeline {
    pub label: String,
    is_initialized: bool,
    pipeline: Option<wgpu::RenderPipeline>,
    layout: Option<wgpu::PipelineLayout>,
    config: RenderPipelineConfig
}

impl std::fmt::Debug for RenderPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderPipeline")
            .field("label", &self.label)
            .field("is_initialized", &self.is_initialized)
            .finish()
    }
}

impl RenderPipeline {
    /// Create a new render pipeline.
    /// By default, the render pipeline does not have a depth or stencil.
    /// By default, the primitive topology is `Topology::TriangleList`.
    /// By default, the cull mode is `Some(Face::Back)`.
    /// By default, the sample count is 1.
    /// By default, this pipeline expects a vertex buffer to be used.
    ///
    /// # Arguments
    ///
    /// * `label` - Label of the render pipeline for debugging.
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            pipeline: None,
            layout: None,
            is_initialized: false,
            config: RenderPipelineConfig {
                depth: DepthDescriptor::default(),
                render_targets: Vec::from([SWAPCHAIN_FORMAT]),
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                push_constants: Vec::new(),
                bind_groups: Vec::new(),
                vertex_shader: String::new(),
                fragment_shader: String::new(),
                fragment_blend: Some(wgpu::BlendState::REPLACE),
                color_write: ColorWrites::ALL,
                cull_mode: Some(Face::Back),
                sample_count: 1,
                use_vertices_buffer: true
            }
        }
    }

    /// Set a given shader.
    ///
    /// # Arguments
    ///
    /// * `shader` - The shader source code.
    /// * `shader_type` - The shader type.
    pub fn set_shader(&mut self, shader: &str, shader_type: ShaderStages) -> &mut Self {
        match shader_type {
            ShaderStages::VERTEX => self.config.vertex_shader = shader.to_string(),
            ShaderStages::FRAGMENT => self.config.fragment_shader = shader.to_string(),
            _ => {
                error!(self.label, "Unsupported shader type.");
            }
        };
        self
    }

    /// Set the primitive topology.
    ///
    /// # Arguments
    ///
    /// * `topology` - The primitive topology.
    pub fn set_topology(&mut self, topology: RenderTopology) -> &mut Self {
        self.config.primitive_topology = match topology {
            RenderTopology::PointList => wgpu::PrimitiveTopology::PointList,
            RenderTopology::LineList => wgpu::PrimitiveTopology::LineList,
            RenderTopology::LineStrip => wgpu::PrimitiveTopology::LineStrip,
            RenderTopology::TriangleList => wgpu::PrimitiveTopology::TriangleList,
            RenderTopology::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip
        };
        self
    }

    /// Set the configuration of the depth/stencil attachment.
    pub fn set_depth(&mut self, depth: DepthDescriptor) -> &mut Self {
        self.config.depth = depth;
        self
    }

    /// Set the cull mode. None means no culling.
    pub fn set_cull_mode(&mut self, cull_mode: Option<Face>) -> &mut Self {
        self.config.cull_mode = cull_mode;
        self
    }

    /// Add a set of bind groups via its layout to the render pipeline.
    /// Note that the order of the bind groups will be the same as the order of the bindings in the shaders.
    ///
    /// # Arguments
    ///
    /// * `layout` - The bind group layout.
    pub fn set_bind_groups(&mut self, layout: Vec<BindGroupLayout>) -> &mut Self {
        for l in layout {
            self.config.bind_groups.push(l);
        }

        self
    }

    /// Set the render targets of the render pipeline.
    ///
    /// # Arguments
    ///
    /// * `targets` - The render targets.
    pub fn set_render_targets(&mut self, targets: Vec<TextureFormat>) -> &mut Self {
        self.config.render_targets = targets;
        self
    }

    /// Set the sample count for multisampling.
    ///
    /// # Arguments
    ///
    /// * `count` - The sample count.
    pub fn set_sample_count(&mut self, count: u32) -> &mut Self {
        self.config.sample_count = count;
        self
    }

    /// Set whether the pipeline uses a vertex buffer.
    ///
    /// # Arguments
    ///
    /// * `use_buffer` - Whether to use a vertex buffer.
    pub fn set_use_vertices_buffer(&mut self, use_buffer: bool) -> &mut Self {
        self.config.use_vertices_buffer = use_buffer;
        self
    }

    /// Set the blend state for the fragment shader.
    ///
    /// # Arguments
    ///
    /// * `blend` - The blend state.
    pub fn set_fragment_blend(&mut self, blend: Option<BlendState>) -> &mut Self {
        self.config.fragment_blend = blend;
        self
    }

    /// Set the color write mask for the fragment output.
    pub fn set_color_write_mask(&mut self, mask: ColorWrites) -> &mut Self {
        self.config.color_write = mask;
        self
    }

    /// Add a push constant to the render pipeline.
    ///
    /// # Arguments
    ///
    /// * `stages` - The shader stages.
    /// * `offset` - The offset of the push constant.
    /// * `size` - The size of the push constant.
    pub fn add_push_constant(&mut self, stages: ShaderStages, offset: u32, size: u32) {
        self.config.push_constants.push(wgpu::PushConstantRange {
            stages,
            range: offset..offset + size
        });
    }

    /// Initialize a new render pipeline.
    ///
    /// # Arguments
    ///
    /// * `instance` - Render instance.
    ///
    /// # Returns
    ///
    /// * `Result<(), RenderError>` - The result of the initialization.
    pub fn init(&mut self, instance: &RenderInstanceData<'_>) -> Result<(), RenderError> {
        let d = &self.config;

        if d.vertex_shader.is_empty() || d.fragment_shader.is_empty() {
            return Err(RenderError::MissingShader);
        }
        let shader_module_vert =
            create_shader_module(instance, &self.label, "vertex", &d.vertex_shader)?;
        let shader_module_frag =
            create_shader_module(instance, &self.label, "fragment", &d.fragment_shader)?;

        // Create pipeline layout
        let layout = validate_pipeline(instance, || {
            instance
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(format!("{}-render-pip-layout", self.label).as_str()),
                    bind_group_layouts: &d
                        .bind_groups
                        .iter()
                        .collect::<Vec<&wgpu::BindGroupLayout>>(),
                    push_constant_ranges: &d.push_constants
                })
        })?;

        // Define vertex buffers
        let vertex_buffers: &[wgpu::VertexBufferLayout] = if d.use_vertices_buffer {
            &[Vertex::describe()]
        } else {
            &[]
        };

        // Create pipeline
        let pipeline = validate_pipeline(instance, || {
            instance
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(format!("{}-render-pip", self.label).as_str()),
                    layout: Some(&layout),
                    cache: None,
                    vertex: wgpu::VertexState {
                        module: &shader_module_vert,
                        entry_point: Some("main"),
                        buffers: vertex_buffers,
                        compilation_options: wgpu::PipelineCompilationOptions::default()
                    },
                    fragment: Some(wgpu::FragmentState {
                        // Always write to swapchain format
                        module: &shader_module_frag,
                        entry_point: Some("main"),
                        targets: d
                            .render_targets
                            .iter()
                            .map(|format| {
                                Some(wgpu::ColorTargetState {
                                    format: *format,
                                    blend: d.fragment_blend,
                                    write_mask: d.color_write
                                })
                            })
                            .collect::<Vec<Option<wgpu::ColorTargetState>>>()
                            .as_slice(),
                        compilation_options: wgpu::PipelineCompilationOptions::default()
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: d.primitive_topology,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: d.cull_mode,
                        polygon_mode: wgpu::PolygonMode::Fill,
                        conservative: false,
                        unclipped_depth: false
                    },
                    depth_stencil: if d.depth.enabled {
                        Some(wgpu::DepthStencilState {
                            format: d.depth.format.unwrap_or(DEPTH_FORMAT),
                            depth_write_enabled: d.depth.write,
                            depth_compare: d.depth.compare,
                            stencil: d.depth.stencil.clone(),
                            bias: wgpu::DepthBiasState::default()
                        })
                    } else {
                        None
                    },
                    multisample: wgpu::MultisampleState {
                        count: d.sample_count,
                        mask: !0,
                        alpha_to_coverage_enabled: false
                    },
                    multiview: Default::default()
                })
        })?;

        // Set pipeline
        self.pipeline = Some(pipeline);
        self.layout = Some(layout);
        self.is_initialized = true;
        Ok(())
    }

    /// Get the render pipeline.
    ///
    /// # Returns
    ///
    /// * `Option<&RenderPipelineRef>` - The render pipeline.
    pub fn get_pipeline(&self) -> Option<&wgpu::RenderPipeline> {
        self.pipeline.as_ref()
    }

    /// Get the pipeline layout.
    ///
    /// # Returns
    ///
    /// * `Option<&PipelineLayout>` - The pipeline layout.
    pub fn get_layout(&self) -> Option<&wgpu::PipelineLayout> {
        self.layout.as_ref()
    }

    /// Check if the render pipeline is initialized.
    ///
    ///
    /// # Returns
    ///
    /// * `bool` - True if the render pipeline is initialized, false otherwise.
    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
