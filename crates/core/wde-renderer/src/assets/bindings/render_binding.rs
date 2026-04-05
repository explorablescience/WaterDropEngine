use std::collections::HashMap;

use wde_logger::prelude::*;
use bevy::{ecs::system::{ScheduleSystem, SystemParamItem, lifetimeless::{SRes, SResMut}}, prelude::*};
use crate::{assets::{bindings::{builder::{RenderBindingBuilderType, RenderBindingsBuilderCache}, dummy_texture::DummyTexture}}, prelude::*};

// Reexport wgpu types
pub use wde_wgpu::buffer::{BufferUsage, BufferBindingType};
pub use wde_wgpu::bind_group::{WgpuBindGroupLayout, BindGroupBuilder, BindGroupLayout, WgpuBindGroup as BindGroup, BindGroupLayoutBuilder};

/// Alias for a `SRes<RenderAssets<GpuRenderBinding<M>>>`.
pub type SBinding<M> = SRes<RenderAssets<GpuRenderBinding<M>>>;

/// Utility resource to store a handle to a render binding asset [RenderBinding].
#[allow(unused)]
#[derive(Resource, Clone)]
pub struct RenderBindingHolder<M: RenderBinding>(pub Handle<M>);

/// Plugin to register a custom [`RenderBinding`] type as an asset, and prepare the corresponding [`GpuRenderBinding`] render asset.
pub struct RenderBindingPluginRegister<M: RenderBinding> {
    _phantom: std::marker::PhantomData<M>,
    default_init: bool
}
impl<M: RenderBinding> RenderBindingPluginRegister<M> {
    /// Create the plugin with a custom initialization system, that can be used to insert the corresponding [`RenderBindingHolder`] resource with a handle to a custom created asset.
    pub fn with_init<S>(init: impl IntoScheduleConfigs<ScheduleSystem, S>, app: &mut App) -> Self {
        app.add_systems(Startup, init);
        Self { _phantom: std::marker::PhantomData, default_init: false }
    }
}
impl<M: RenderBinding> Default for RenderBindingPluginRegister<M> {
    /// Create the plugin with a default initialization, that inserts the corresponding [`RenderBindingHolder`] resource with a handle to a default created asset.
    fn default() -> Self { RenderBindingPluginRegister { _phantom: std::marker::PhantomData, default_init: true } }
}
impl<M: RenderBinding + Default> Plugin for RenderBindingPluginRegister<M> {
    fn build(&self, app: &mut App) {
        app
            .init_asset::<M>()
            .add_plugins(RenderAssetsPlugin::<GpuRenderBinding<M>>::default());
    }
    fn finish(&self, app: &mut App) {
        if self.default_init {
            let binding: Handle<M> = app.world_mut().add_asset(M::default());
            app.get_sub_app_mut(RenderApp).unwrap()
                .insert_resource(RenderBindingHolder(binding));
        }
    }
}

/// A render binding is a collection of GPU resources (buffers, textures, samplers) that are bound together in a bind group and used by shaders to render objects. It is an abstraction layer over the [wde_wgpu::bind_group] API.
/// The `describe` method is used to declare the resources that will be extracted from the render binding and uploaded to the GPU when preparing the corresponding [`GpuRenderBinding`] asset.
pub trait RenderBinding: Asset + Clone + Sized {
    /// Describe buffers, textures and samplers that make up this render bind group.
    /// Use the provided [`RenderBindingBuilder`] to add entries at specific bindings that correspond to the shader's expected layout.
    fn describe(&self, builder: &mut RenderBindingBuilder);
    fn label(&self) -> &str { std::any::type_name::<Self>() }
}

/// Represents a GPU material asset prepared from a [`RenderBinding`].
/// It contains the bind group, as well as the builder used to create it, that contains the original buffers/textures.
/// To retrieve a buffer or texture handle from the group, use the `get_buffer` and `get_texture` methods with the corresponding binding.
pub struct GpuRenderBinding<M: RenderBinding> {
    _phantom: std::marker::PhantomData<M>,
    builder: RenderBindingBuilder,
    builder_bindings_to_index: HashMap<u32, usize>,

    /// Bind group layout matching the render binding's declared bindings.
    pub layout: BindGroupLayout,
    /// Bind group populated with buffers/textures referenced by the render binding.
    pub bind_group: BindGroup
}
impl<M: RenderBinding> GpuRenderBinding<M> {
    /// Get the GPU buffer asset id for a given binding, if it exists.
    pub fn get_buffer(&self, binding: u32) -> Option<AssetId<Buffer>> {
        if let Some(index) = self.builder_bindings_to_index.get(&binding)
            && let RenderBindingBuilderType::Buffer = self.builder.elements[*index].0 {
            return self.builder.buffers[*index].2.as_ref().map(|handle| handle.id());
        }
        None
    }
    /// Get the GPU texture asset id for a given binding, if it exists.
    pub fn get_texture(&self, binding: u32) -> Option<AssetId<Texture>> {
        if let Some(index) = self.builder_bindings_to_index.get(&binding) {
            match self.builder.elements[*index].0 {
                RenderBindingBuilderType::TextureView => {
                    return self.builder.texture_views[*index].texture.as_ref().map(|handle| handle.id());
                }
                RenderBindingBuilderType::TextureSampler => {
                    return self.builder.texture_samplers[*index].texture.as_ref().map(|handle| handle.id());
                }
                _ => {}
            }
        }
        None
    }
}
impl<M: RenderBinding> RenderAsset for GpuRenderBinding<M> {
    type SourceAsset = M;
    type Params = (
        SRes<RenderInstance>, SResMut<RenderBindingsBuilderCache>, SRes<AssetServer>,
        SRes<DummyTexture>, SRes<RenderAssets<GpuBuffer>>, SRes<RenderAssets<GpuTexture>>
    );

    fn prepare(
            asset: Self::SourceAsset,
            (render_instance, renderbinding_cache, assets_server, dummy_texture, buffers, textures): &mut SystemParamItem<Self::Params>
        ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let render_instance = render_instance.0.read().unwrap();
        let label = asset.label();
        let binding_label = format!("{}-{}", std::any::type_name::<M>(), label);

        // Get or create render bind groups builder
        let mut bindings_builder = if let Some(builder) = renderbinding_cache.remove(&binding_label) {
            builder
        } else {
            let mut builder = RenderBindingBuilder::default();
            asset.describe(&mut builder);
            builder
        };

        // Create bind group entries. If a buffer or texture is not ready, retry next update
        let mut bg_entries = Vec::new();
        let mut bindings_to_index = HashMap::new();
        for (binding_type, binding_index) in &bindings_builder.elements {
            match binding_type {
                RenderBindingBuilderType::Buffer => {
                    let buffer = &bindings_builder.buffers[*binding_index as usize];
                    bindings_to_index.insert(buffer.0, *binding_index as usize);

                    // Create buffer if not already loaded on cpu
                    if buffer.2.is_none() {
                        let bf_handle = assets_server.add(buffer.1.clone());
                        bindings_builder.buffers[*binding_index as usize].1.content = None;
                        bindings_builder.buffers[*binding_index as usize].2 = Some(bf_handle);
                        renderbinding_cache.insert(binding_label.to_string(), bindings_builder);
                        return Err(PrepareAssetError::RetryNextUpdate(asset));
                    }

                    // Check if buffer loaded on gpu
                    if let Some(bf) = buffers.get(buffer.2.as_ref().unwrap()) {
                        bg_entries.push(BindGroupBuilder::buffer(buffer.0, &bf.buffer));
                    } else {
                        renderbinding_cache.insert(binding_label.to_string(), bindings_builder);
                        return Err(PrepareAssetError::RetryNextUpdate(asset));
                    }
                }
                RenderBindingBuilderType::TextureView => {
                    let texture = &bindings_builder.texture_views[*binding_index as usize];
                    bindings_to_index.insert(texture.binding, *binding_index as usize);

                    if let Some(ref texture_handle) = texture.texture {
                        if let Some(tex) = textures.get(texture_handle) {
                            bg_entries.push(BindGroupBuilder::texture_view(texture.binding, &tex.texture));
                        } else {
                            renderbinding_cache.insert(binding_label.to_string(), bindings_builder);
                            return Err(PrepareAssetError::RetryNextUpdate(asset));
                        }
                    }
                    else {
                        // Set dummy texture
                        bindings_builder.texture_views[*binding_index as usize].texture = Some(dummy_texture.0.clone());
                        renderbinding_cache.insert(binding_label.to_string(), bindings_builder);
                        return Err(PrepareAssetError::RetryNextUpdate(asset));
                    }
                }
                RenderBindingBuilderType::TextureSampler => {
                    let texture = &bindings_builder.texture_samplers[*binding_index as usize];
                    bindings_to_index.insert(texture.binding, *binding_index as usize);

                    if let Some(ref texture_handle) = texture.texture {
                        if let Some(tex) = textures.get(texture_handle) {
                            bg_entries.push(BindGroupBuilder::texture_sampler(texture.binding, &tex.texture));
                        } else {
                            renderbinding_cache.insert(binding_label.to_string(), bindings_builder);
                            return Err(PrepareAssetError::RetryNextUpdate(asset));
                        }
                    }
                    else {
                        // Set dummy texture
                        bindings_builder.texture_samplers[*binding_index as usize].texture = Some(dummy_texture.0.clone());
                        renderbinding_cache.insert(binding_label.to_string(), bindings_builder);
                        return Err(PrepareAssetError::RetryNextUpdate(asset));
                    }
                }
            }
        }

        // Create bind group layout
        let mut is_err = false;
        let layout = BindGroupLayout::new(label, |builder| {
            for (material_type, material_index) in &bindings_builder.elements {
                match material_type {
                    RenderBindingBuilderType::Buffer => {
                        let buffer = &bindings_builder.buffers[*material_index as usize];
                        let binding_type = if buffer.1.usage.contains(BufferUsage::UNIFORM) {
                            BufferBindingType::Uniform
                        } else if buffer.1.usage.contains(BufferUsage::STORAGE) {
                            BufferBindingType::Storage { read_only: true }
                        } else {
                            error!("Buffer at binding {} has no usage flag, defaulting to UNIFORM.", buffer.0);
                            BufferBindingType::Uniform
                        };
                        builder.add_buffer(buffer.0, ShaderStages::all(), binding_type);
                    }
                    RenderBindingBuilderType::TextureView => {
                        let view = &bindings_builder.texture_views[*material_index as usize];
                        if let Some(ref texture_handle) = view.texture
                            && let Some(tex) = textures.get(texture_handle) {
                            builder.add_texture_view(view.binding, view.visibility, tex.texture.sample_count > 1);
                        } else {
                            is_err = true;
                        }
                    }
                    RenderBindingBuilderType::TextureSampler => {
                        let sampler = &bindings_builder.texture_samplers[*material_index as usize];
                        builder.add_texture_sampler(sampler.binding, sampler.visibility);
                    }
                }
            }
        });
        if is_err {
            renderbinding_cache.insert(binding_label.to_string(), bindings_builder);
            return Err(PrepareAssetError::RetryNextUpdate(asset));
        }

        // Create layout
        let layout_built = match layout.build(&render_instance) {
            Ok(layout) => layout,
            Err(_) => {
                return Err(PrepareAssetError::RetryNextUpdate(asset));
            }
        };

        // Create bind group
        let bind_group = match BindGroupBuilder::build(label, &render_instance, &layout_built, &bg_entries) {
            Ok(bind_group) => bind_group,
            Err(_) => {
                return Err(PrepareAssetError::RetryNextUpdate(asset));
            }
        };

        // Return GPU asset
        Ok(GpuRenderBinding {
            _phantom: std::marker::PhantomData,
            layout,
            bind_group,
            builder: bindings_builder,
            builder_bindings_to_index: bindings_to_index
        })
    }

    fn label(&self) -> &str { &self.builder.label }
}
