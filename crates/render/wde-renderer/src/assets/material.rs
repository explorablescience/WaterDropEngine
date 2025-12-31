use std::collections::HashMap;

use bevy::{ecs::system::lifetimeless::{SRes, SResMut}, prelude::*};
use wde_wgpu::{bind_group::{BindGroup, BindGroupLayout, BufferBindingType, WgpuBindGroup}, buffer::BufferUsage, render_pipeline::ShaderStages, texture::{TextureFormat, TextureUsages}};

use crate::core::{RenderApp, RenderInstance};

use super::{Buffer, GpuBuffer, GpuTexture, PrepareAssetError, RenderAsset, RenderAssets, RenderAssetsPlugin, Texture, TextureLoaderSettings};

pub trait Material {
    /// Describe the material by adding buffers, textures, etc. to the material builder.
    fn describe(&self, builder: &mut MaterialBuilder);
    /// Get the label of the material
    fn label(&self) -> String;
}


struct MaterialBuilderBuffer {
    binding: u32,
    visibility: ShaderStages,
    binding_type: BufferBindingType,
    size: usize,
    content: Option<Vec<u8>>,
    buffer: Option<Handle<Buffer>>
}
struct MaterialBuilderTextureView {
    binding: u32,
    visibility: ShaderStages,
    texture: Option<Handle<Texture>>
}
struct MaterialBuilderTextureSampler {
    binding: u32,
    visibility: ShaderStages,
    texture: Option<Handle<Texture>>
}

enum MaterialBuilderType {
    Buffer,
    TextureView,
    TextureSampler
}

#[derive(Default)]
pub struct MaterialBuilder {
    label: String,
    elements: Vec<(MaterialBuilderType, u32)>,

    buffers: Vec<MaterialBuilderBuffer>,
    texture_views: Vec<MaterialBuilderTextureView>,
    texture_samplers: Vec<MaterialBuilderTextureSampler>
}
impl MaterialBuilder {
    /// Add a buffer to the material. A buffer is a uniform or storage buffer that will be created by the material builder.
    pub fn add_buffer(&mut self, binding: u32, visibility: ShaderStages, binding_type: BufferBindingType, size: usize, content: Option<Vec<u8>>) {
        self.buffers.push(MaterialBuilderBuffer {
            binding, visibility, binding_type, size, content, buffer: None
        });
        self.elements.push((MaterialBuilderType::Buffer, self.buffers.len() as u32 - 1));
    }
    pub fn add_texture_view(&mut self, binding: u32, visibility: ShaderStages, texture: Option<Handle<Texture>>) {
        self.texture_views.push(MaterialBuilderTextureView {
            binding, visibility, texture
        });
        self.elements.push((MaterialBuilderType::TextureView, self.texture_views.len() as u32 - 1));
    }
    pub fn add_texture_sampler(&mut self, binding: u32, visibility: ShaderStages, texture: Option<Handle<Texture>>) {
        self.texture_samplers.push(MaterialBuilderTextureSampler {
            binding, visibility, texture
        });
        self.elements.push((MaterialBuilderType::TextureSampler, self.texture_samplers.len() as u32 - 1));
    }
}


#[derive(Default, Resource)]
pub struct MaterialsBuilderCache {
    materials: HashMap<String, MaterialBuilder>
}
impl MaterialsBuilderCache {
    fn remove(&mut self, label: &str) -> Option<MaterialBuilder> {
        self.materials.remove(label)
    }
    fn insert(&mut self, label: String, material: MaterialBuilder) {
        self.materials.insert(label, material);
    }
}

#[derive(Resource)]
pub struct DummyTexture(Handle<Texture>);

pub struct GpuMaterial<M: Material + Sync + Send + Asset + Clone> {
    phantom: std::marker::PhantomData<M>,
    builder: MaterialBuilder,
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: WgpuBindGroup
}
impl<M: Material + Sync + Send + Asset + Clone> RenderAsset for GpuMaterial<M> {
    type SourceAsset = M;
    type Param = (
        SRes<RenderInstance<'static>>, SResMut<MaterialsBuilderCache>, SRes<AssetServer>,
        SRes<DummyTexture>, SRes<RenderAssets<GpuBuffer>>, SRes<RenderAssets<GpuTexture>>
    );

    fn prepare_asset(
            asset: Self::SourceAsset,
            (render_instance, materials_cache, assets_server, dummy_texture, buffers, textures):
                &mut bevy::ecs::system::SystemParamItem<Self::Param>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let render_instance = render_instance.0.read().unwrap();
        let label = asset.label();
        let material_name = format!("{}-{}", std::any::type_name::<M>(), label);

        // Get or create material builder
        let mut material_builder = if let Some(builder) = materials_cache.remove(&material_name) {
            builder
        } else {
            let mut builder = MaterialBuilder::default();
            asset.describe(&mut builder);
            builder
        };

        // Create bind group entries
        // If a buffer or texture is not ready, retry next update
        let mut bg_entries = Vec::new();
        for (material_type, material_index) in &material_builder.elements {
            match material_type {
                MaterialBuilderType::Buffer => {
                    let buffer = &material_builder.buffers[*material_index as usize];

                    // Create buffer if not already loaded on cpu
                    if buffer.buffer.is_none() {
                        let bf_handle = assets_server.add(Buffer {
                            label: label.to_string(),
                            size: buffer.size,
                            usage: match buffer.binding_type {
                                BufferBindingType::Uniform => BufferUsage::UNIFORM,
                                BufferBindingType::Storage { .. } => BufferUsage::STORAGE
                            },
                            content: buffer.content.clone()
                        });
                        material_builder.buffers[*material_index as usize].content = None;
                        material_builder.buffers[*material_index as usize].buffer = Some(bf_handle);
                        materials_cache.insert(material_name.to_string(), material_builder);
                        return Err(PrepareAssetError::RetryNextUpdate(asset));
                    }

                    // Check if buffer loaded on gpu
                    if let Some(bf) = buffers.get(buffer.buffer.as_ref().unwrap()) {
                        bg_entries.push(BindGroup::buffer(buffer.binding, &bf.buffer));
                    } else {
                        materials_cache.insert(material_name.to_string(), material_builder);
                        return Err(PrepareAssetError::RetryNextUpdate(asset));
                    }
                }
                MaterialBuilderType::TextureView => {
                    let texture = &material_builder.texture_views[*material_index as usize];
                    if let Some(ref texture_handle) = texture.texture {
                        if let Some(tex) = textures.get(texture_handle) {
                            bg_entries.push(BindGroup::texture_view(texture.binding, &tex.texture));
                        } else {
                            materials_cache.insert(material_name.to_string(), material_builder);
                            return Err(PrepareAssetError::RetryNextUpdate(asset));
                        }
                    }
                    else {
                        // Set dummy texture
                        material_builder.texture_views[*material_index as usize].texture = Some(dummy_texture.0.clone());
                        materials_cache.insert(material_name.to_string(), material_builder);
                        return Err(PrepareAssetError::RetryNextUpdate(asset));
                    }
                }
                MaterialBuilderType::TextureSampler => {
                    let texture = &material_builder.texture_samplers[*material_index as usize];
                    if let Some(ref texture_handle) = texture.texture {
                        if let Some(tex) = textures.get(texture_handle) {
                            bg_entries.push(BindGroup::texture_sampler(texture.binding, &tex.texture));
                        } else {
                            materials_cache.insert(material_name.to_string(), material_builder);
                            return Err(PrepareAssetError::RetryNextUpdate(asset));
                        }
                    }
                    else {
                        // Set dummy texture
                        material_builder.texture_samplers[*material_index as usize].texture = Some(dummy_texture.0.clone());
                        materials_cache.insert(material_name.to_string(), material_builder);
                        return Err(PrepareAssetError::RetryNextUpdate(asset));
                    }
                }
            }
        }

        // Create bind group layout
        let layout = BindGroupLayout::new(&label, |builder| {
            for (material_type, material_index) in &material_builder.elements {
                match material_type {
                    MaterialBuilderType::Buffer => {
                        let buffer = &material_builder.buffers[*material_index as usize];
                        builder.add_buffer(buffer.binding, buffer.visibility, buffer.binding_type);
                    }
                    MaterialBuilderType::TextureView => {
                        let view = &material_builder.texture_views[*material_index as usize];
                        builder.add_texture_view(view.binding, view.visibility);
                    }
                    MaterialBuilderType::TextureSampler => {
                        let sampler = &material_builder.texture_samplers[*material_index as usize];
                        builder.add_texture_sampler(sampler.binding, sampler.visibility);
                    }
                }
            }
        });

        // Create bind group
        let bind_group = BindGroup::build(&label, &render_instance, &layout.build(&render_instance), &bg_entries);

        Ok(GpuMaterial {
            phantom: std::marker::PhantomData,
            bind_group_layout: layout,
            bind_group,
            builder: material_builder
        })
    }

    fn label(&self) -> &str {
        &self.builder.label
    }
}




pub struct MaterialsPluginRegister<M: Material + Sync + Send + Asset + Clone> {
    phantom: std::marker::PhantomData<M>
}
impl<M: Material + Sync + Send + Asset + Clone> Default for MaterialsPluginRegister<M> {
    fn default() -> Self {
        MaterialsPluginRegister {
            phantom: std::marker::PhantomData
        }
    }
}
impl<M: Material + Sync + Send + Asset + Clone> Plugin for MaterialsPluginRegister<M> {
    fn build(&self, app: &mut App) {
        app
            .init_asset::<M>()
            .add_plugins(RenderAssetsPlugin::<GpuMaterial<M>>::default());
    }
}


pub(crate) struct MaterialsPluginRaw;
impl Plugin for MaterialsPluginRaw {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        // Load the dummy texture
        let assets_server = app.world().get_resource::<AssetServer>().unwrap();
        let dummy_texture = assets_server.load_with_settings("pbr/dummy_texture.png",
        |settings: &mut TextureLoaderSettings| {
            settings.label = "dummy-texture".to_string();
            settings.format = TextureFormat::R8Unorm;
            settings.usages = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
        });
        
        app.get_sub_app_mut(RenderApp).unwrap()
            .insert_resource(DummyTexture(dummy_texture));
    }
}
