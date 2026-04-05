//! Bind groups bind buffers, textures, and samplers into a shader-visible layout.
//!
//! # Overview
//! 1. Create a [`BindGroupLayout`] that matches your WGSL `@group` / `@binding` declarations.
//! 2. Populate GPU resources and build the concrete [`wgpu::BindGroup`].
//! 3. Set the bind group inside a render or compute pass.
//!
//! # Example: PBR material
//! ```rust,no_run
//! use wde_wgpu::{
//!     bind_group::*,
//!     buffer::{Buffer, BufferUsage},
//!     instance::RenderInstanceData,
//!     render_pipeline::ShaderStages,
//!     texture::{Texture, TextureFormat, TextureUsages},
//! };
//! 
//! let uniform_buffer = Buffer::new(instance, "material-uniform", 64, BufferUsage::UNIFORM, None);
//! let albedo = Texture::new(instance, "albedo", (512, 512), TextureFormat::Rgba8Unorm, TextureUsages::TEXTURE_BINDING, 1, 1);
//! let normal = Texture::new(instance, "normal", (512, 512), TextureFormat::Rgba8Unorm, TextureUsages::TEXTURE_BINDING, 1, 1);
//!
//! let layout = BindGroupLayout::new("material-layout", |builder| {
//!     builder.add_buffer(0, ShaderStages::FRAGMENT, BufferBindingType::Uniform);
//!     builder.add_texture_view(1, ShaderStages::FRAGMENT);
//!     builder.add_texture_sampler(2, ShaderStages::FRAGMENT);
//!     builder.add_texture_view(3, ShaderStages::FRAGMENT);
//!     builder.add_texture_sampler(4, ShaderStages::FRAGMENT);
//! });
//! let wgpu_layout = layout.build(instance);
//! 
//! let bind_group = BindGroup::build(
//!     "material-bind-group",
//!     instance,
//!     &wgpu_layout,
//!     &vec![
//!         BindGroup::buffer(0, &uniform_buffer),
//!         BindGroup::texture_view(1, &albedo),
//!         BindGroup::texture_sampler(2, &albedo),
//!         BindGroup::texture_view(3, &normal),
//!         BindGroup::texture_sampler(4, &normal),
//!     ],
//! );
//! 
//! (wgpu_layout, bind_group)
//! ```
//!
//! Depth-only layouts follow the same pattern using `add_depth_texture_view` and
//! `add_depth_texture_sampler`.
//!
//! # Tips
//! - Keep per-frame data in group 0, per-material in group 1, and per-object in group 2 to
//!   minimize rebinding during draws.
//! - The order of `set_bind_groups` on pipelines must match the order of layouts you provide
//!   here.

use futures_lite::future;
use wde_logger::prelude::*;

use crate::{buffer::Buffer, instance::{RenderError, RenderInstanceData}, render_pipeline::ShaderStages, texture::{Texture, TextureFormat}};

/// The wgpu bind group layout builder.
pub type WgpuBindGroup = wgpu::BindGroup;

// The wgpu bind group type.
pub type BindGroup = wgpu::BindGroup;

/// The buffer binding type.
pub type BufferBindingType = wgpu::BufferBindingType;

/// The wgpu bind group layout type.
pub type WgpuBindGroupLayout = wgpu::BindGroupLayout;

/// The wgpu bind group entry type.
pub type BindGroupEntry<'a> = wgpu::BindGroupEntry<'a>;

/// Builder for a bind group layout.
#[derive(Debug, Clone)]
pub struct BindGroupLayoutBuilder {
    layout_entries: Vec<wgpu::BindGroupLayoutEntry>,
}

impl BindGroupLayoutBuilder {
    /// Add a buffer to the bind group.
    /// 
    /// # Arguments
    /// 
    /// * `binding` - The binding index of the buffer.
    /// * `visibility` - The shader stages that can access the buffer.
    /// * `binding_type` - The type of the buffer binding.
    pub fn add_buffer(&mut self, binding: u32, visibility: ShaderStages, binding_type: BufferBindingType) -> &mut Self {
        // Create bind group layout
        self.layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                has_dynamic_offset: false,
                min_binding_size: None,
                ty: binding_type,
            },
            count: None,
        });

        self
    }

    /// Add a texture to the bind group.
    /// 
    /// # Arguments
    /// 
    /// * `binding` - The binding index of the texture.
    /// * `visibility` - The shader stages that can access the texture.
    /// * `multisampled` - Whether the texture is multisampled.
    pub fn add_texture_view(&mut self, binding: u32, visibility: ShaderStages, multisampled: bool) -> &mut Self {
        // Create bind group layout
        self.layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Texture {
                multisampled,
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: !multisampled },
            },
            count: None
        });

        self
    }

    /// Add a storage texture view to the bind group.
    /// This is used for textures that will be read and written to in a compute shader.
    /// 
    /// # Arguments
    /// 
    /// * `binding` - The binding index of the texture.
    /// * `format` - The format of the texture.
    /// * `atomic` - Whether the texture will be used for atomic operations.
    pub fn add_storage_texture_view(&mut self, binding: u32, format: TextureFormat) -> &mut Self {
        // Create bind group layout
        self.layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::COMPUTE,
            ty: wgpu::BindingType::StorageTexture {
                access: wgpu::StorageTextureAccess::ReadWrite,
                view_dimension: wgpu::TextureViewDimension::D2,
                format
            },
            count: None
        });

        self
    }

    /// Add a texture array to the bind group.
    /// 
    /// # Arguments
    /// 
    /// * `binding` - The binding index of the texture array.
    /// * `visibility` - The shader stages that can access the texture array.
    pub fn add_texture_array_view(&mut self, binding: u32, visibility: ShaderStages) -> &mut Self {
        // Create bind group layout
        self.layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2Array,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            },
            count: None
        });

        self
    }

    /// Add a depth texture to the bind group.
    ///
    /// # Arguments
    /// 
    /// * `binding` - The binding index of the texture.
    /// * `visibility` - The shader stages that can access the texture.
    /// * `multisampled` - Whether the texture is multisampled.
    pub fn add_depth_texture_view(&mut self, binding: u32, visibility: ShaderStages, multisampled: bool) -> &mut Self {
        // Create bind group layout
        self.layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Texture {
                multisampled,
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Depth,
            },
            count: None
        });

        self
    }

    /// Add a texture to the bind group.
    /// 
    /// # Arguments
    /// 
    /// * `binding` - The binding index of the texture sampler.
    /// * `visibility` - The shader stages that can access the sampler.
    pub fn add_texture_sampler(&mut self, binding: u32, visibility: ShaderStages) -> &mut Self {
        self.layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });

        self
    }

    /// Add a depth texture sampler to the bind group.
    /// 
    /// # Arguments
    /// 
    /// * `binding` - The binding index of the texture sampler.
    /// * `visibility` - The shader stages that can access the sampler.
    pub fn add_depth_texture_sampler(&mut self, binding: u32, visibility: ShaderStages) -> &mut Self {
        self.layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });

        self
    }
}

/// Structure for a bind group layout.
/// Stores the layout description and builder data.
#[derive(Clone)]
pub struct BindGroupLayout {
    // Bind group description
    pub label: String,
    // Access to builder data
    pub builder: BindGroupLayoutBuilder,
}

impl BindGroupLayout {
    /// Create a new bind group layout.
    /// 
    /// # Arguments
    /// 
    /// * `label` - The label of the bind group layout. Note that this label is only for GPU debugging purposes and is only used if GPU debugging is enabled.
    /// * `build_func` - The function to build the bind group layout.
    pub fn new(label: &str, build_func: impl FnOnce(&mut BindGroupLayoutBuilder)) -> Self {
        let mut builder = BindGroupLayoutBuilder {
            layout_entries: Vec::new(),
        };

        build_func(&mut builder);

        BindGroupLayout {
            label: label.to_string(),
            builder,
        }
    }

    /// Build the bind group layout.
    /// 
    /// # Arguments
    /// 
    /// * `instance` - The render instance data.
    pub fn build(&self, instance: &RenderInstanceData) -> Result<WgpuBindGroupLayout, RenderError> {
        event!(Level::TRACE, "Creating bind group layout: {}.", self.label);

        // Add validation to intercept potential errors in bind group layout creation
        instance.device.push_error_scope(wgpu::ErrorFilter::Validation);

        // Create bind group layout
        let layout = instance.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some(format!("{}-bind-group-layout", self.label).as_str()),
                entries: &self.builder.layout_entries,
            }
        );

        // Check for errors
        let mut res = Ok(layout);
        future::block_on(async {
            let error = instance.device.pop_error_scope().await;
            match error {
                Some(wgpu::Error::Validation { source, description }) => {
                    error!(self.label, "Failed to create bind group layout with source error: {:#?}. Description: {}.", source, description);
                    res = Err(RenderError::CannotCreateBindGroupLayout);
                },
                Some(e) => {
                    error!(self.label, "Failed to create bind group layout with unexpected error: {:#?}.", e);
                    res = Err(RenderError::CannotCreateBindGroupLayout);
                },
                None => (),
            }
        });
        res
    }
}

/// Structure for a bind group.
pub struct BindGroupBuilder;
impl BindGroupBuilder {
    /// Build a bind group.
    /// 
    /// # Arguments
    /// 
    /// * `label` - The label of the bind group. Note that this label is only for GPU debugging purposes and is only used if GPU debugging is enabled.
    /// * `instance` - The render instance data.
    /// * `layout` - The bind group layout.
    /// * `entries` - The bind group entries.
    pub fn build(label: &str, instance: &RenderInstanceData, layout: &wgpu::BindGroupLayout, entries: &Vec<wgpu::BindGroupEntry>) -> Result<wgpu::BindGroup, RenderError> {
        event!(Level::TRACE, "Creating bind group: {}.", label);

        // Add validation to intercept potential errors in bind group creation
        instance.device.push_error_scope(wgpu::ErrorFilter::Validation);

        // Create bind group
        let bind_group =instance.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(format!("{}-bind-group", label).as_str()),
                layout,
                entries
            }
        );

        // Check for errors
        let mut res = Ok(bind_group);
        future::block_on(async {
            let error = instance.device.pop_error_scope().await;
            match error {
                Some(wgpu::Error::Validation { source, description }) => {
                    error!(label, "Failed to create bind group with source error: {:#?}. Description: {}.", source, description);
                    res = Err(RenderError::CannotCreateBindGroup);
                },
                Some(e) => {
                    error!(label, "Failed to create bind group with unexpected error: {:#?}.", e);
                    res = Err(RenderError::CannotCreateBindGroup);
                },
                None => (),
            }
        });
        res
    }

    /// Add a buffer to the bind group.
    /// 
    /// # Arguments
    /// 
    /// * `binding` - The binding index of the buffer.
    /// * `buffer` - The buffer to add to the bind group.
    pub fn buffer(binding: u32, buffer: &'_ Buffer) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: buffer.buffer.as_entire_binding(),
        }
    }

    /// Add a texture view to the bind group.
    /// 
    /// # Arguments
    /// 
    /// * `binding` - The binding index of the texture.
    /// * `texture` - The texture to add to the bind group.
    pub fn texture_view(binding: u32, texture: &'_ Texture) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(&texture.view),
        }
    }
    
    /// Add a texture sampler to the bind group.
    /// 
    /// # Arguments
    /// 
    /// * `binding` - The binding index of the texture.
    /// * `texture` - The texture to add to the bind group.
    pub fn texture_sampler(binding: u32, texture: &'_ Texture) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(&texture.sampler),
        }
    }
}
