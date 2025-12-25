//! Compute pipeline module.

use bevy::{log::{trace, Level}, prelude::*, log::tracing::event};
use wgpu::{naga, BindGroupLayout, ShaderStages};

use crate::instance::{WRenderError, WRenderInstanceData};

// Compute pipeline configuration
struct WComputePipelineConfig {
    push_constants: Vec<wgpu::PushConstantRange>,
    bind_groups: Vec<wgpu::BindGroupLayout>,
    shader: String
}


/// Create a new compute pipeline.
/// 
/// First, we need to create a new bind group describing the resources that will be used in the compute pipeline.
/// See the [BindGroup](struct@crate::bind_group::BindGroup) struct for more information.
/// 
/// ```rust
/// // Create a new compute pipeline
/// let mut pipeline = WComputePipeline::new("Compute Pipeline");
/// pipeline
///    .set_shader(include_str!("[...].comp"))   // Set the compute shader
///    .add_push_constant(4)                     // Say that we will provide push constant at offset 0 with size 4
///    .add_bind_group(bind_group.layout)        // Say that we will use a bind group
///    .init(&instance);                         // Initialize the pipeline
/// 
/// // Check if the pipeline is initialized
/// if pipeline.is_initialized() {
///    // Get the compute pipeline
///    let compute_pipeline = pipeline.get_pipeline().unwrap();
///    
///    // Get the pipeline layout
///    let layout = pipeline.get_layout().unwrap();
/// }
/// ```
pub struct WComputePipeline {
    /// Label for the compute pipeline
    pub label: String,
    /// The compute pipeline
    pub pipeline: Option<wgpu::ComputePipeline>,
    /// The pipeline layout
    pub layout: Option<wgpu::PipelineLayout>,
    /// Whether the compute pipeline has been initialized
    pub is_initialized: bool,
    /// Configuration of the compute pipeline
    config: WComputePipelineConfig,
}

impl WComputePipeline {
    /// Create a new compute pipeline.
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
            config: WComputePipelineConfig {
                push_constants: Vec::new(),
                bind_groups: Vec::new(),
                shader: String::new()
            },
        }
    }

    /// Set the compute shader of the pipeline.
    /// 
    /// # Arguments
    /// 
    /// * `shader` - The shader source code.
    pub fn set_shader(&mut self, shader: &str) -> &mut Self {
        self.config.shader = shader.to_string();
        self
    }

    /// Add a set of bind groups via its layout to the compute pipeline.
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

    /// Add a push constant to the compute pipeline.
    /// 
    /// # Arguments
    /// 
    /// * `size` - The size of the push constant.
    pub fn add_push_constant(&mut self, size: u32) -> &mut Self {
        self.config.push_constants.push(wgpu::PushConstantRange {
            stages : ShaderStages::COMPUTE,
            range: 0..size,
        });
        self
    }

    /// Initialize a new compute pipeline.
    /// 
    /// # Arguments
    /// 
    /// * `instance` - Render instance.
    /// 
    /// # Returns
    /// 
    /// * `Result<(), RenderError>` - The result of the initialization.
    pub fn init(&mut self, instance: &WRenderInstanceData) -> Result<(), WRenderError> {
        event!(Level::DEBUG, "Creating compute pipeline {}.", self.label);
        let d = &self.config;

        // Security checks
        if d.shader.is_empty() {
            error!(self.label, "Pipeline does not have a compute shader.");
            return Err(WRenderError::MissingShader);
        }
        
        // Load shader
        trace!(self.label, "Loading compute shader.");
        let shader_module = match naga::front::wgsl::parse_str(&self.config.shader) {
            Ok(shader) => {
                match naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all(),
                ).validate(&shader) {
                    Ok(_) => {
                        instance.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                            label: Some(format!("{}-render-pip-comp", self.label).as_str()),
                            source: wgpu::ShaderSource::Wgsl(self.config.shader.to_owned().into()),
                        })
                    },
                    Err(e) => {
                        error!(self.label, "Compute shader validation failed: {:?}.", e);
                        return Err(WRenderError::ShaderCompilationError);
                    }
                }
            },
            Err(e) => {
                let mut error = format!("Compute shader parsing failed \"{}\".\n", e);
                for (span, message) in e.labels() {
                    let location = span.location(&self.config.shader);
                    error.push_str(&format!(" - Error on line {} at position {}: \"{}\"\n", location.line_number, location.line_position, message));
                }
                error!(self.label, "{}", error);
                return Err(WRenderError::ShaderCompilationError);
            }
        };

        // Create pipeline layout
        trace!(self.label, "Creating compute pipeline instance.");
        let layout = instance.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(format!("{}-compute-pip-layout", self.label).as_str()),
            bind_group_layouts: &d.bind_groups.iter().collect::<Vec<&wgpu::BindGroupLayout>>(),
            push_constant_ranges: &d.push_constants,
        });

        // Create a compute pipeline
        let pipeline = instance.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(format!("{}-compute-pip", self.label).as_str()),
            layout: Some(&layout),
            module: &shader_module,
            entry_point: "main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None
        });

        // Set pipeline
        self.pipeline = Some(pipeline);
        self.layout = Some(layout);
        self.is_initialized = true;

        Ok(())
    }


    /// Get the compute pipeline.
    ///
    /// # Returns
    /// 
    /// * `Option<&RenderPipelineRef>` - The compute pipeline.
    pub fn get_pipeline(&self) -> Option<&wgpu::ComputePipeline> {
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

    /// Check if the compute pipeline is initialized.
    ///
    /// 
    /// # Returns
    /// 
    /// * `bool` - True if the compute pipeline is initialized, false otherwise.
    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
