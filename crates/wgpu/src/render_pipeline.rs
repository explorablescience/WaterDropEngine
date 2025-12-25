//! Render pipeline module.

use bevy::{log::{error, trace, Level}, log::tracing::event};
use wgpu::{naga, BindGroupLayout};

use crate::{instance::{WRenderError, WRenderInstanceData}, texture::{WTexture, WTextureFormat}, vertex::WVertex};

/// List of available shaders.
pub type WShaderStages = wgpu::ShaderStages;
/// Type of the shader module.
pub type WShaderModule = wgpu::ShaderModule;
/// Export culling params.
pub type WFace = wgpu::Face;
/// Export compare function.
pub type WCompareFunction = wgpu::CompareFunction;

/// Describes the depth/stencil attachment of a render pipeline.
#[derive(Clone)]
pub struct WDepthStencilDescriptor {
    /// Whether the pipeline should have a depth/stencil attachment.
    pub enabled: bool,
    /// Whether the stencil attachment should be read-only.
    pub write: bool,
    /// The comparison function that the depth attachment will use.
    pub compare: WCompareFunction
}
impl Default for WDepthStencilDescriptor {
    fn default() -> Self {
        Self {
            enabled: false,
            write: true,
            compare: WCompareFunction::Less
        }
    }
}

/// List of available topologies.
#[derive(Clone, Copy)]
pub enum WTopology {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
}

// Render pipeline configuration
struct WRenderPipelineConfig {
    depth: WDepthStencilDescriptor,
    render_targets: Vec<WTextureFormat>,
    primitive_topology: wgpu::PrimitiveTopology,
    push_constants: Vec<wgpu::PushConstantRange>,
    bind_groups: Vec<wgpu::BindGroupLayout>,
    vertex_shader: String,
    fragment_shader: String,
    cull_mode: Option<WFace>,
}


/// Stores a render pipeline
/// 
/// # Example
/// 
/// ```
/// let mut pipeline = WRenderPipeline::new("...");
/// pipeline
///     .set_shader(include_str!("[...].vert"), WShaderType::Vertex)   // Set the vertex shader
///     .set_shader(include_str!("[...].frag"), WShaderType::Fragment) // Set the fragment shader
///     .set_topology(WTopology::LineList)            // Change the primitive topology
///     .set_depth_stencil()                         // Enable depth and stencil
///     .add_push_constant(WShaderType::Vertex, 0, 4) // Say that we will provide push constant at offset 0 with size 4
///     .add_bind_group(bind_group_layout)           // Say that we will use a bind group
///     .init(&instance);                            // Initialize the pipeline
/// 
/// if pipeline.is_initialized() {
///    // Use the pipeline
///    let pipeline = pipeline.get_pipeline().unwrap();
///    let layout = pipeline.get_layout().unwrap();
/// }
/// ```
pub struct WRenderPipeline {
    pub label: String,
    is_initialized: bool,
    pipeline: Option<wgpu::RenderPipeline>,
    layout: Option<wgpu::PipelineLayout>,
    config: WRenderPipelineConfig,
}

impl std::fmt::Debug for WRenderPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderPipeline")
            .field("label", &self.label)
            .field("is_initialized", &self.is_initialized)
            .finish()
    }
}

impl WRenderPipeline {
    /// Create a new render pipeline.
    /// By default, the render pipeline does not have a depth or stencil.
    /// By default, the primitive topology is `Topology::TriangleList`.
    /// By default, the cull mode is `Some(Face::Back)`.
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
            config: WRenderPipelineConfig {
                depth: WDepthStencilDescriptor::default(),
                render_targets: Vec::from([WTexture::SWAPCHAIN_FORMAT]),
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                push_constants: Vec::new(),
                bind_groups: Vec::new(),
                vertex_shader: String::new(),
                fragment_shader: String::new(),
                cull_mode: Some(WFace::Back),
            },
        }
    }

    /// Set a given shader.
    /// 
    /// # Arguments
    /// 
    /// * `shader` - The shader source code.
    /// * `shader_type` - The shader type.
    pub fn set_shader(&mut self, shader: &str, shader_type: WShaderStages) -> &mut Self {
        match shader_type {
            WShaderStages::VERTEX => self.config.vertex_shader = shader.to_string(),
            WShaderStages::FRAGMENT => self.config.fragment_shader = shader.to_string(),
            _ => { error!(self.label, "Unsupported shader type."); }
        };
        self
    }

    /// Set the primitive topology.
    /// 
    /// # Arguments
    /// 
    /// * `topology` - The primitive topology.
    pub fn set_topology(&mut self, topology: WTopology) -> &mut Self {
        self.config.primitive_topology = match topology {
            WTopology::PointList => wgpu::PrimitiveTopology::PointList,
            WTopology::LineList => wgpu::PrimitiveTopology::LineList,
            WTopology::LineStrip => wgpu::PrimitiveTopology::LineStrip,
            WTopology::TriangleList => wgpu::PrimitiveTopology::TriangleList,
            WTopology::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
        };
        self
    }

    /// Set the configuration of the depth/stencil attachment.
    pub fn set_depth(&mut self, depth: WDepthStencilDescriptor) -> &mut Self {
        self.config.depth = depth;
        self
    }

    /// Set the cull mode. None means no culling.
    pub fn set_cull_mode(&mut self, cull_mode: Option<WFace>) -> &mut Self {
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
    pub fn set_render_targets(&mut self, targets: Vec<WTextureFormat>) -> &mut Self {
        self.config.render_targets = targets;
        self
    }

    /// Add a push constant to the render pipeline.
    /// 
    /// # Arguments
    /// 
    /// * `stages` - The shader stages.
    /// * `offset` - The offset of the push constant.
    /// * `size` - The size of the push constant.
    pub fn add_push_constant(&mut self, stages: WShaderStages, offset: u32, size: u32) {
        self.config.push_constants.push(wgpu::PushConstantRange {
            stages,
            range: offset..offset + size,
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
    pub fn init(&mut self, instance: &WRenderInstanceData<'_>) -> Result<(), WRenderError> {
        event!(Level::DEBUG, "Creating render pipeline {}.", self.label);
        let d = &self.config;

        // Security checks
        if d.vertex_shader.is_empty() || d.fragment_shader.is_empty() {
            error!(self.label, "Pipeline does not have a vertex or fragment shader.");
            return Err(WRenderError::MissingShader);
        }
        
        // Load vertex shader
        trace!(self.label, "Loading shaders.");
        let shader_module_vert = match naga::front::wgsl::parse_str(&self.config.vertex_shader) {
            Ok(shader) => {
                match naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all(),
                ).validate(&shader) {
                    Ok(_) => {
                        instance.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                            label: Some(format!("{}-render-pip-vert", self.label).as_str()),
                            source: wgpu::ShaderSource::Wgsl(self.config.vertex_shader.to_owned().into()),
                        })
                    },
                    Err(e) => {
                        error!(self.label, "Vertex shader validation failed: {:?}.", e);
                        return Err(WRenderError::ShaderCompilationError);
                    }
                }
            },
            Err(e) => {
                let mut error = format!("Vertex shader parsing failed \"{}\".\n", e);
                for (span, message) in e.labels() {
                    let location = span.location(&self.config.vertex_shader);
                    error.push_str(&format!(" - Error on line {} at position {}: \"{}\"\n", location.line_number, location.line_position, message));
                }
                error!(self.label, "{}", error);
                return Err(WRenderError::ShaderCompilationError);
            }
        };

        // Load fragment shader
        let shader_module_frag = match naga::front::wgsl::parse_str(&self.config.fragment_shader) {
            Ok(shader) => {
                match naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all(),
                ).validate(&shader) {
                    Ok(_) => {
                        instance.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                            label: Some(format!("{}-render-pip-frag", self.label).as_str()),
                            source: wgpu::ShaderSource::Wgsl(self.config.fragment_shader.to_owned().into()),
                        })
                    },
                    Err(e) => {
                        error!(self.label, "Fragment shader validation failed: {:?}.", e);
                        return Err(WRenderError::ShaderCompilationError);
                    }
                }
            },
            Err(e) => {
                let mut error = format!("Fragment shader parsing failed \"{}\".\n", e);
                for (span, message) in e.labels() {
                    let location = span.location(&self.config.fragment_shader);
                    error.push_str(&format!(" - Error on line {} at position {}: \"{}\"\n", location.line_number, location.line_position, message));
                }
                error!(self.label, "{}", error);
                return Err(WRenderError::ShaderCompilationError);
            }
        };

        // Create pipeline layout
        trace!(self.label, "Creating render pipeline instance.");
        let layout = instance.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(format!("{}-render-pip-layout", self.label).as_str()),
            bind_group_layouts: &d.bind_groups.iter().collect::<Vec<&wgpu::BindGroupLayout>>(),
            push_constant_ranges: &d.push_constants,
        });

        // Create pipeline
        let mut res: Result<(), WRenderError> = Ok(());
        let pipeline = instance.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(format!("{}-render-pip", self.label).as_str()),
            layout: Some(&layout),
            cache: None,
            vertex: wgpu::VertexState {
                module: &shader_module_vert,
                entry_point: "main",
                buffers: &[WVertex::describe()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState { // Always write to swapchain format
                module: &shader_module_frag,
                entry_point: "main",
                targets: d.render_targets.iter().map(|format| Some(wgpu::ColorTargetState {
                    format: *format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })).collect::<Vec<Option<wgpu::ColorTargetState>>>().as_slice(),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: d.primitive_topology,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: d.cull_mode,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: if d.depth.enabled { Some(wgpu::DepthStencilState {
                format: match WTexture::DEPTH_FORMAT {
                    WTextureFormat::Depth32Float => wgpu::TextureFormat::Depth32Float,
                    _ => {
                        error!("Depth format is not supported for render pipeline '{}'.", self.label);
                        res = Err(WRenderError::UnsupportedDepthFormat);
                        wgpu::TextureFormat::Depth32Float
                    }
                },
                depth_write_enabled: d.depth.write,
                depth_compare: d.depth.compare,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }) } else { None },
            multisample: wgpu::MultisampleState::default(),
            multiview: Default::default(),
        });

        // Set pipeline
        self.pipeline = Some(pipeline);
        self.layout = Some(layout);
        self.is_initialized = true;

        res
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
