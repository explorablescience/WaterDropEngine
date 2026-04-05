//! Compute pipeline module.
use futures_lite::future;
use wde_logger::prelude::*;
use wgpu::{BindGroupLayout, ShaderStages, naga};

use crate::instance::{RenderError, RenderInstanceData};

// Compute pipeline configuration
struct ComputePipelineConfig {
    push_constants: Vec<wgpu::PushConstantRange>,
    bind_groups: Vec<wgpu::BindGroupLayout>,
    shader: String
}

/// Create a compute pipeline from WGSL source and bind group layouts.
///
/// # Example
/// ```rust,no_run
/// use wde_wgpu::compute_pipeline::ComputePipeline;
///
/// let mut pipeline = ComputePipeline::new("particles");
/// pipeline
///     .set_shader(include_str!("../../../res/marching-cubes/spawn_terrain.comp.wgsl"))
///     .set_bind_groups(layouts)
///     .add_push_constant(16) // bytes starting at offset 0
///     .init(instance)
///     .expect("shader validated");
/// assert!(pipeline.is_initialized());
///
/// let _wgpu_pipeline = pipeline.get_pipeline().unwrap();
/// ```
pub struct ComputePipeline {
    /// Label for the compute pipeline
    pub label: String,
    /// The compute pipeline
    pub pipeline: Option<wgpu::ComputePipeline>,
    /// The pipeline layout
    pub layout: Option<wgpu::PipelineLayout>,
    /// Whether the compute pipeline has been initialized
    pub is_initialized: bool,
    /// Configuration of the compute pipeline
    config: ComputePipelineConfig
}

impl ComputePipeline {
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
            config: ComputePipelineConfig {
                push_constants: Vec::new(),
                bind_groups: Vec::new(),
                shader: String::new()
            }
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
    /// * `offset` - The offset of the push constant.
    /// * `size` - The size of the push constant.
    pub fn add_push_constant(&mut self, offset: u32, size: u32) -> &mut Self {
        self.config.push_constants.push(wgpu::PushConstantRange {
            stages: ShaderStages::COMPUTE,
            range: offset..offset + size
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
    pub fn init(&mut self, instance: &RenderInstanceData) -> Result<(), RenderError> {
        event!(Level::TRACE, "Creating compute pipeline {}.", self.label);
        let d = &self.config;

        // Security checks
        if d.shader.is_empty() {
            error!(self.label, "Pipeline does not have a compute shader.");
            return Err(RenderError::MissingShader);
        }

        // Load shader
        trace!(self.label, "Loading compute shader.");
        let shader_module = match naga::front::wgsl::parse_str(&self.config.shader) {
            Ok(shader) => {
                match naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all()
                )
                .validate(&shader)
                {
                    Ok(_) => instance
                        .device
                        .create_shader_module(wgpu::ShaderModuleDescriptor {
                            label: Some(format!("{}-render-pip-comp", self.label).as_str()),
                            source: wgpu::ShaderSource::Wgsl(self.config.shader.to_owned().into())
                        }),
                    Err(e) => {
                        error!(self.label, "Compute shader validation failed: {:?}.", e);
                        return Err(RenderError::ShaderCompilationError);
                    }
                }
            }
            Err(e) => {
                let mut error = format!("Compute shader parsing failed \"{}\".\n", e);
                for (span, message) in e.labels() {
                    let location = span.location(&self.config.shader);
                    error.push_str(&format!(
                        " - Error on line {} at position {}: \"{}\"\n",
                        location.line_number, location.line_position, message
                    ));
                }
                error!(self.label, "{}", error);
                return Err(RenderError::ShaderCompilationError);
            }
        };
        future::block_on(async {
            let compil_info = shader_module.get_compilation_info().await;
            for message in compil_info.messages {
                match message.message_type {
                    wgpu::CompilationMessageType::Error => error!(
                        self.label,
                        "Compute shader {} compilation error '{}' (at {:?}).",
                        self.label,
                        message.message,
                        message.location
                    ),
                    wgpu::CompilationMessageType::Warning => warn!(
                        self.label,
                        "Compute shader {} compilation warning '{}' (at {:?}).",
                        self.label,
                        message.message,
                        message.location
                    ),
                    wgpu::CompilationMessageType::Info => debug!(
                        self.label,
                        "Compute shader {} compilation info '{}' (at {:?}).",
                        self.label,
                        message.message,
                        message.location
                    )
                }
            }
        });

        // Create pipeline layout
        trace!(self.label, "Creating compute pipeline instance.");
        let layout = instance
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(format!("{}-compute-pip-layout", self.label).as_str()),
                bind_group_layouts: &d
                    .bind_groups
                    .iter()
                    .collect::<Vec<&wgpu::BindGroupLayout>>(),
                push_constant_ranges: &d.push_constants
            });

        // Create a compute pipeline
        instance
            .device
            .push_error_scope(wgpu::ErrorFilter::Validation);
        let pipeline = instance
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(format!("{}-compute-pip", self.label).as_str()),
                layout: Some(&layout),
                module: &shader_module,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None
            });

        // Check for errors
        let mut res: Result<(), RenderError> = Ok(());
        future::block_on(async {
            let error = instance.device.pop_error_scope().await;
            match error {
                Some(wgpu::Error::Validation {
                    source,
                    description
                }) => {
                    error!(
                        self.label,
                        "Failed to create compute pipeline with source error: {:?}. Description: {}.",
                        source,
                        description
                    );
                    res = Err(RenderError::ShaderCompilationError);
                }
                Some(e) => {
                    error!(self.label, "Failed to create compute pipeline: {:?}.", e);
                    res = Err(RenderError::ShaderCompilationError);
                }
                None => ()
            }
        });

        // Set pipeline
        self.pipeline = Some(pipeline);
        self.layout = Some(layout);
        self.is_initialized = true;

        res
    }

    /// Get the compute pipeline.
    ///
    /// # Returns
    ///
    /// * `Option<&ComputePipelineRef>` - The compute pipeline.
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
