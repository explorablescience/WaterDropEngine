//! Bind groups are used to bind resources to shaders.

use bevy::{log::Level, log::tracing::event};

use crate::{buffer::WBuffer, instance::WRenderInstanceData, render_pipeline::WShaderStages, texture::WTexture};

/// The wgpu bind group layout builder.
pub type WgpuBindGroup = wgpu::BindGroup;

/// The buffer binding type.
pub type WBufferBindingType = wgpu::BufferBindingType;

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
    pub fn add_buffer(&mut self, binding: u32, visibility: WShaderStages, binding_type: WBufferBindingType) -> &mut Self {
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
    /// * `binding` - The binding index of the texture. Note that the binding index of the sampler is incremented by 1
    /// * `visibility` - The shader stages that can access the texture.
    pub fn add_texture_view(&mut self, binding: u32, visibility: WShaderStages) -> &mut Self {
        // Create bind group layout
        self.layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2,
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
    /// * `binding` - The binding index of the texture. Note that the binding index of the sampler is incremented by 1
    /// * `visibility` - The shader stages that can access the texture.
    pub fn add_depth_texture_view(&mut self, binding: u32, visibility: WShaderStages) -> &mut Self {
        // Create bind group layout
        self.layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
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
    /// * `binding` - The binding index of the texture. Note that the binding index of the sampler is incremented by 1
    /// * `visibility` - The shader stages that can access the texture.
    pub fn add_texture_sampler(&mut self, binding: u32, visibility: WShaderStages) -> &mut Self {
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
    /// * `binding` - The binding index of the texture. Note that the binding index of the sampler is incremented by 1
    /// * `visibility` - The shader stages that can access the texture.
    pub fn add_depth_texture_sampler(&mut self, binding: u32, visibility: WShaderStages) -> &mut Self {
        self.layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });

        self
    }
}


pub type WgpuBindGroupLayout = wgpu::BindGroupLayout;

#[derive(Clone)]
pub struct BindGroupLayout {
    // Bind group description
    pub label: String,
    // Access to builder data
    pub builder: BindGroupLayoutBuilder,
}

impl BindGroupLayout {
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

    pub fn build(&self, instance: &WRenderInstanceData) -> wgpu::BindGroupLayout {
        event!(Level::TRACE, "Creating bind group layout: {}.", self.label);

        // Create bind group layout
        instance.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some(format!("{}-bg-layout", self.label).as_str()),
                entries: &self.builder.layout_entries,
            }
        )
    }
}


pub type WBindGroupEntry<'a> = wgpu::BindGroupEntry<'a>;

/// Structure for a bind group.
pub struct BindGroup;
impl BindGroup {
    pub fn build(label: &str, instance: &WRenderInstanceData, layout: &wgpu::BindGroupLayout, entries: &Vec<wgpu::BindGroupEntry>) -> wgpu::BindGroup {
        event!(Level::TRACE, "Creating bind group: {}.", label);

        // Create bind group
        instance.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(format!("{}-bg", label).as_str()),
                layout,
                entries
            }
        )
    }

    /// Add a buffer to the bind group.
    /// 
    /// # Arguments
    /// 
    /// * `binding` - The binding index of the buffer.
    /// * `buffer` - The buffer to add to the bind group.
    pub fn buffer(binding: u32, buffer: &WBuffer) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding,
            resource: buffer.buffer.as_entire_binding(),
        }
    }

    /// Add a texture view to the bind group.
    /// 
    /// # Arguments
    /// 
    /// * `binding` - The binding index of the texture. Note that the binding index of the sampler is incremented by 1.
    /// * `texture` - The texture to add to the bind group.
    pub fn texture_view(binding: u32, texture: &WTexture) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(&texture.view),
        }
    }
    
    /// Add a texture sampler to the bind group.
    /// 
    /// # Arguments
    /// 
    /// * `binding` - The binding index of the texture. Note that the binding index of the sampler is incremented by 1.
    /// * `texture` - The texture to add to the bind group.
    pub fn texture_sampler(binding: u32, texture: &WTexture) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(&texture.sampler),
        }
    }
}
