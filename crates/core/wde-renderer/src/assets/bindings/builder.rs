use std::collections::HashMap;

use crate::prelude::*;
use bevy::prelude::*;
use wde_wgpu::pipelines::ShaderStages;

pub(crate) struct RenderBindingBuilderTextureView {
    pub binding: u32,
    pub visibility: ShaderStages,
    pub texture: Option<Handle<Texture>>
}
pub(crate) struct RenderBindingBuilderTextureSampler {
    pub binding: u32,
    pub visibility: ShaderStages,
    pub texture: Option<Handle<Texture>>
}
pub(crate) enum RenderBindingBuilderType {
    Buffer,
    TextureView,
    TextureSampler
}

/// Utility to collect buffers/textures and binding metadata for a RenderBinding.
#[derive(Default)]
pub struct RenderBindingBuilder {
    pub(crate) label: String,
    pub(crate) no_bind_group: bool,
    pub(crate) elements: Vec<(RenderBindingBuilderType, u32)>,

    pub(crate) buffers: Vec<(u32, Buffer, Option<Handle<Buffer>>)>,
    pub(crate) texture_views: Vec<RenderBindingBuilderTextureView>,
    pub(crate) texture_samplers: Vec<RenderBindingBuilderTextureSampler>
}
impl RenderBindingBuilder {
    pub fn add_buffer(&mut self, binding: u32, buffer: Buffer) {
        self.buffers.push((binding, buffer, None));
        self.elements.push((
            RenderBindingBuilderType::Buffer,
            self.buffers.len() as u32 - 1
        ));
    }
    pub fn add_texture_view(&mut self, binding: u32, texture: Option<Handle<Texture>>) {
        self.texture_views.push(RenderBindingBuilderTextureView {
            binding,
            visibility: ShaderStages::all(),
            texture
        });
        self.elements.push((
            RenderBindingBuilderType::TextureView,
            self.texture_views.len() as u32 - 1
        ));
    }
    pub fn add_texture_sampler(&mut self, binding: u32, texture: Option<Handle<Texture>>) {
        self.texture_samplers
            .push(RenderBindingBuilderTextureSampler {
                binding,
                visibility: ShaderStages::all(),
                texture
            });
        self.elements.push((
            RenderBindingBuilderType::TextureSampler,
            self.texture_samplers.len() as u32 - 1
        ));
    }
    /// Indicates that this render binding doesn't need to create a bind group.
    pub fn no_bind_group(&mut self) {
        self.no_bind_group = true;
    }
}

/// Cache of partially built render binding descriptions waiting for GPU resources.
#[derive(Default, Resource)]
pub struct RenderBindingsBuilderCache {
    renderbindings: HashMap<String, RenderBindingBuilder>
}
impl RenderBindingsBuilderCache {
    pub fn remove(&mut self, label: &str) -> Option<RenderBindingBuilder> {
        self.renderbindings.remove(label)
    }
    pub fn insert(&mut self, label: String, material: RenderBindingBuilder) {
        self.renderbindings.insert(label, material);
    }
}
