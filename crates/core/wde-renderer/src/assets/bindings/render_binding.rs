use wde_logger::prelude::*;

use crate::{assets::bindings::render_data::RenderDataRecreated, prelude::*};
use bevy::{
    ecs::system::{
        ReadOnlySystemParam, SystemParamItem, SystemState,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use std::{any::TypeId, collections::HashSet, marker::PhantomData};
use wde_wgpu::pipelines::{BindGroup, BindGroupBuilder, BindGroupLayout};

// ============= RENDER DATA PLUGIN REGISTER =============
#[allow(unused)]
#[derive(Resource)]
struct RenderBindingHolder<R: RenderBinding + TypePath + Sync + Send + Asset>(Handle<R>);
impl<R: RenderBinding + TypePath + Sync + Send + Asset> Clone for RenderBindingHolder<R> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<R: RenderBinding + TypePath + Sync + Send + Asset> ExtractResource for RenderBindingHolder<R> {
    type Source = Self;

    fn extract(source: &Self::Source) -> Self {
        source.clone()
    }
}

/// A plugin to register a render binding.
///
/// This should be added to the app to initialize the [`RenderBinding`] asset and create the
/// corresponding [`GpuRenderBinding`].
///
/// It also tracks [`RenderData`](crate::assets::bindings::RenderData) dependencies declared in
/// [`RenderBinding::describe`], and recreates the binding asset when one of those dependencies
/// is recreated.
///
/// The render binding type must implement [`RenderBinding`] and usually derive [`Asset`],
/// [`TypePath`], [`Clone`] and [`Default`].
pub struct RenderBindingRegisterPlugin<R: RenderBinding>(std::marker::PhantomData<R>);
impl<R: RenderBinding> Default for RenderBindingRegisterPlugin<R> {
    fn default() -> Self {
        RenderBindingRegisterPlugin(std::marker::PhantomData)
    }
}
impl<R: RenderBinding + TypePath + Sync + Send + Clone + Asset + Default> Plugin
    for RenderBindingRegisterPlugin<R>
{
    fn build(&self, app: &mut App) {
        app.init_asset::<R>()
            .add_plugins((
                RenderAssetsPlugin::<GpuRenderBinding<R>>::default(),
                ExtractResourcePlugin::<RenderBindingHolder<R>>::default()
            ))
            .add_systems(Update, on_dependency_recreate::<R>);
    }

    fn finish(&self, app: &mut App) {
        let mut default_res = R::default();

        // Register dependencies
        let builder = {
            let world = app.get_sub_app_mut(RenderApp).unwrap().world_mut();
            let mut builder = RenderBindingBuilder::default();
            let mut state: SystemState<R::Params> = SystemState::new(world);
            let params = state.get_mut(world);
            R::describe(&mut default_res, &params, &mut builder);
            builder
        };
        app.world_mut()
            .insert_resource(RenderBindingDependencies::<R> {
                _phantom: PhantomData,
                dependencies: builder.dependencies
            });

        // Create the asset and insert the handle in the resource
        let binding: Handle<R> = app.world_mut().add_asset(default_res);
        app.world_mut()
            .insert_resource(RenderBindingHolder(binding));
    }
}

#[derive(Resource)]
struct RenderBindingDependencies<R: RenderBinding> {
    _phantom: PhantomData<R>,
    dependencies: HashSet<TypeId>
}
fn on_dependency_recreate<R: RenderBinding + Asset + Default>(
    asset_server: Res<AssetServer>,
    mut dependency_update: MessageReader<RenderDataRecreated>,
    mut render_binding_holder: ResMut<RenderBindingHolder<R>>,
    mut update_next_tick: Local<Option<TypeId>>,
    dependencies: Res<RenderBindingDependencies<R>>
) {
    for message in dependency_update.read() {
        let type_id = message.0;

        // Only update next tick
        // This is as first tick, the render data asset will be recreated
        // So only second tick can be update the render binding asset, to be sure the render data asset is ready
        if update_next_tick.is_none() {
            *update_next_tick = Some(type_id);
            return;
        }
    }

    if let Some(type_id) = *update_next_tick {
        *update_next_tick = None;

        // Check if the render binding depends on the render data that have been recreated
        if !dependencies.dependencies.contains(&type_id) {
            return;
        }

        // Recreate asset
        debug!(
            "Recreating render binding {} because of dependency render data change.",
            std::any::type_name::<R>()
        );
        render_binding_holder.0 = asset_server.add(R::default());
    }
}

// ============= RENDER BINDING AND BUILDER METHODS =============
enum RenderBindingType {
    Buffer,
    TextureView,
    TextureArrayView,
    TextureSampler,
    StorageTexture
}

/// A builder for the render binding. This is used in the description of the render binding to specify the buffers and textures that should be created on the gpu, and to build the bind group layout and bind group entries.
#[derive(Default)]
pub struct RenderBindingBuilder {
    is_none: bool, // True if there is at least one buffer or texture that is not ready
    elements: Vec<(RenderBindingType, u32)>, // (binding type, buffer/texture index in the corresponding vector)
    buffers: Vec<Option<AssetId<Buffer>>>,
    textures: Vec<Option<AssetId<Texture>>>,
    dependencies: HashSet<TypeId>
}
impl RenderBindingBuilder {
    /// Adds a buffer to the render binding. The `render_data` parameter is the [`RenderData`](RenderData) that contains the buffer, and the `render_data_idx` parameter is the index of the buffer in this render data.
    pub fn add_buffer<R>(
        &mut self,
        render_data: &RenderAssets<GpuRenderData<R>>,
        render_data_idx: u32
    ) -> &mut Self
    where
        R: RenderData + Clone + Asset + 'static
    {
        self.dependencies.insert(TypeId::of::<R>());
        let buffer = render_data
            .iter()
            .next()
            .and_then(|(_, d)| d.get_buffer(render_data_idx))
            .map(|b| b.id());
        if buffer.is_none() {
            self.is_none = true;
        }
        self.buffers.push(buffer);
        self.elements
            .push((RenderBindingType::Buffer, self.buffers.len() as u32 - 1));
        self
    }
    /// Adds a buffer directly from an asset id.
    pub fn add_buffer_from_id(&mut self, buffer: Option<AssetId<Buffer>>) -> &mut Self {
        if buffer.is_none() {
            self.is_none = true;
        }
        self.buffers.push(buffer);
        self.elements
            .push((RenderBindingType::Buffer, self.buffers.len() as u32 - 1));
        self
    }
    /// Adds a texture view to the render binding. The `render_data` parameter is the [`RenderData`](RenderData) that contains the texture, and the `render_data_idx` parameter is the index of the texture in this render data.
    pub fn add_texture_view<R>(
        &mut self,
        render_data: &RenderAssets<GpuRenderData<R>>,
        render_data_idx: u32
    ) -> &mut Self
    where
        R: RenderData + Clone + Asset + 'static
    {
        self.add_texture(render_data, render_data_idx, RenderBindingType::TextureView)
    }
    /// Adds a texture view directly from an asset id.
    pub fn add_texture_view_from_id(&mut self, texture: Option<AssetId<Texture>>) -> &mut Self {
        self.add_texture_from_id(texture, RenderBindingType::TextureView)
    }
    /// Adds a texture array view to the render binding. The `render_data` parameter is the [`RenderData`](RenderData) that contains the texture, and the `render_data_idx` parameter is the index of the texture in this render data.
    pub fn add_texture_array_view<R>(
        &mut self,
        render_data: &RenderAssets<GpuRenderData<R>>,
        render_data_idx: u32
    ) -> &mut Self
    where
        R: RenderData + Clone + Asset + 'static
    {
        self.add_texture(
            render_data,
            render_data_idx,
            RenderBindingType::TextureArrayView
        )
    }
    /// Adds a texture array view directly from an asset id.
    pub fn add_texture_array_view_from_id(&mut self, texture: Option<AssetId<Texture>>) -> &mut Self {
        self.add_texture_from_id(texture, RenderBindingType::TextureArrayView)
    }
    /// Adds a texture sampler to the render binding. The `render_data` parameter is the [`RenderData`](RenderData) that contains the texture, and the `render_data_idx` parameter is the index of the texture in this render data.
    pub fn add_texture_sampler<R>(
        &mut self,
        render_data: &RenderAssets<GpuRenderData<R>>,
        render_data_idx: u32
    ) -> &mut Self
    where
        R: RenderData + Clone + Asset + 'static
    {
        self.add_texture(
            render_data,
            render_data_idx,
            RenderBindingType::TextureSampler
        )
    }
    /// Adds a texture sampler directly from an asset id.
    pub fn add_texture_sampler_from_id(&mut self, texture: Option<AssetId<Texture>>) -> &mut Self {
        self.add_texture_from_id(texture, RenderBindingType::TextureSampler)
    }
    /// Adds a storage texture view to the render binding. The `render_data` parameter is the [`RenderData`](RenderData) that contains the texture, and the `render_data_idx` parameter is the index of the texture in this render data.
    pub fn add_storage_texture<R>(
        &mut self,
        render_data: &RenderAssets<GpuRenderData<R>>,
        render_data_idx: u32
    ) -> &mut Self
    where
        R: RenderData + Clone + Asset + 'static
    {
        self.add_texture(
            render_data,
            render_data_idx,
            RenderBindingType::StorageTexture
        )
    }
    /// Adds a storage texture view directly from an asset id.
    pub fn add_storage_texture_from_id(&mut self, texture: Option<AssetId<Texture>>) -> &mut Self {
        self.add_texture_from_id(texture, RenderBindingType::StorageTexture)
    }

    fn add_texture<R>(
        &mut self,
        render_data: &RenderAssets<GpuRenderData<R>>,
        render_data_idx: u32,
        binding_type: RenderBindingType
    ) -> &mut Self
    where
        R: RenderData + Clone + Asset + 'static
    {
        self.dependencies.insert(TypeId::of::<R>());
        let texture = render_data
            .iter()
            .next()
            .and_then(|(_, d)| d.get_texture(render_data_idx))
            .map(|t| t.id());
        if texture.is_none() {
            self.is_none = true;
        }
        self.textures.push(texture);
        self.elements
            .push((binding_type, self.textures.len() as u32 - 1));
        self
    }
    fn add_texture_from_id(&mut self, texture: Option<AssetId<Texture>>, binding_type: RenderBindingType) -> &mut Self {
        if texture.is_none() {
            self.is_none = true;
        }
        self.textures.push(texture);
        self.elements
            .push((binding_type, self.textures.len() as u32 - 1));
        self
    }
}

/// A trait for describing a render binding.
///
/// This trait is used to declare bind-group entries by referencing resources from
/// [`RenderData`](crate::assets::bindings::RenderData). Register the type using
/// [`RenderBindingRegisterPlugin`] and retrieve the prepared GPU side through
/// [`GpuRenderBinding`] in render systems.
pub trait RenderBinding {
    type Params: ReadOnlySystemParam;

    /// Describes the render binding. This is used to specify the [`RenderData`](RenderData) that should be used to create the bind group layout and bind group entries, and to specify the binding index for each buffer and texture.
    fn describe(&mut self, params: &SystemParamItem<Self::Params>, builder: &mut RenderBindingBuilder);
    /// Whether the gpu asset should be recreated. This is called every frame. The default is to never recreate (None).
    fn label(&self) -> &str;
}

// ============= RENDER BINDING RENDER ASSET GPU CREATION =============
/// The gpu asset that is created from the render binding holder. This is the asset that is actually used in the render pass.
pub type SRenderBinding<T> = SRes<RenderAssets<GpuRenderBinding<T>>>;
/// The gpu asset that is created from the render binding holder. This is the asset that is actually used in the render pass.
pub type SRenderBindingMut<T> = SResMut<RenderAssets<GpuRenderBinding<T>>>;

/// The gpu asset that is created from the render binding holder. This is the asset that is actually used in the render pass.
/// It contains the bind group and bind group layout that are created according to the description in the [`RenderBinding`](RenderBinding) implementation, and can be retrieved using the binding index specified in the description.
pub struct GpuRenderBinding<T> {
    _phantom: PhantomData<T>,
    pub layout: BindGroupLayout,
    pub bind_group: BindGroup
}
impl<T> RenderAsset for GpuRenderBinding<T>
where
    T: RenderBinding + TypePath + Sync + Send + Clone + Asset
{
    type SourceAsset = T;
    type Params = (
        T::Params,
        SRes<RenderInstance>,
        SRes<RenderAssets<GpuBuffer>>,
        SRes<RenderAssets<GpuTexture>>
    );

    fn prepare(
        asset: Self::SourceAsset,
        (binding_params, render_instance, gpu_buffers, gpu_textures): &mut SystemParamItem<
            Self::Params
        >
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        // Clone the asset to be able to modify it in the description
        let mut asset = asset.clone();

        // Describe the asset
        let mut asset_builder = RenderBindingBuilder::default();
        T::describe(&mut asset, binding_params, &mut asset_builder);
        if asset_builder.is_none {
            // If any of the buffers or textures were not ready, retry in the next update
            trace!(
                "Not all resources for render binding {} are ready, retrying...",
                asset.label()
            );
            return Err(PrepareAssetError::RetryNextUpdate(asset));
        }

        // Check every texture and buffers are ready
        for (binding_type, idx) in &asset_builder.elements {
            match binding_type {
                RenderBindingType::Buffer => {
                    if gpu_buffers
                        .get(asset_builder.buffers[*idx as usize].unwrap())
                        .is_none()
                    {
                        trace!(
                            "Buffer for render binding {} is not ready, retrying...",
                            asset.label()
                        );
                        return Err(PrepareAssetError::RetryNextUpdate(asset));
                    }
                }
                RenderBindingType::TextureView
                | RenderBindingType::TextureArrayView
                | RenderBindingType::TextureSampler
                | RenderBindingType::StorageTexture => {
                    if gpu_textures
                        .get(asset_builder.textures[*idx as usize].unwrap())
                        .is_none()
                    {
                        trace!(
                            "Texture for render binding {} is not ready, retrying...",
                            asset.label()
                        );
                        return Err(PrepareAssetError::RetryNextUpdate(asset));
                    };
                }
            }
        }

        // Create the bind group layout
        let mut is_err = false;
        let layout = BindGroupLayout::new(asset.label(), |builder| {
            let vis = ShaderStages::all();
            for (binding, (binding_type, idx)) in asset_builder.elements.iter().enumerate() {
                match binding_type {
                    RenderBindingType::Buffer => {
                        let Some(buffer) =
                            gpu_buffers.get(asset_builder.buffers[*idx as usize].unwrap())
                        else {
                            trace!(
                                "Buffer for render binding {} is not ready, retrying...",
                                asset.label()
                            );
                            is_err = true;
                            return;
                        };
                        let binding_type = if buffer
                            .buffer
                            .buffer
                            .usage()
                            .contains(BufferUsage::UNIFORM)
                        {
                            BufferBindingType::Uniform
                        } else if buffer.buffer.buffer.usage().contains(BufferUsage::STORAGE) {
                            BufferBindingType::Storage { read_only: true }
                        } else {
                            error!(
                                "Buffer has no usage flag, defaulting to UNIFORM for render binding {}.",
                                asset.label()
                            );
                            BufferBindingType::Uniform
                        };
                        builder.add_buffer(binding as u32, vis, binding_type)
                    }
                    RenderBindingType::TextureView => {
                        let Some(texture) =
                            gpu_textures.get(asset_builder.textures[*idx as usize].unwrap())
                        else {
                            trace!(
                                "Texture for render binding {} is not ready, retrying...",
                                asset.label()
                            );
                            is_err = true;
                            return;
                        };
                        let multisampled = texture.texture.sample_count > 1;
                        builder.add_texture_view(binding as u32, vis, multisampled)
                    }
                    RenderBindingType::TextureArrayView => {
                        builder.add_texture_array_view(binding as u32, vis)
                    }
                    RenderBindingType::TextureSampler => {
                        builder.add_texture_sampler(binding as u32, vis)
                    }
                    RenderBindingType::StorageTexture => {
                        let Some(texture) =
                            gpu_textures.get(asset_builder.textures[*idx as usize].unwrap())
                        else {
                            trace!(
                                "Texture for render binding {} is not ready, retrying...",
                                asset.label()
                            );
                            is_err = true;
                            return;
                        };
                        builder.add_storage_texture_view(binding as u32, texture.texture.format)
                    }
                };
            }
        });
        if is_err {
            return Err(PrepareAssetError::RetryNextUpdate(asset));
        }

        // Build the bind group layout
        let render_instance = render_instance.0.read().unwrap();
        let layout_built = match layout.build(&render_instance) {
            Ok(layout) => layout,
            Err(_) => return Err(PrepareAssetError::RetryNextUpdate(asset))
        };

        // Create the bind group
        let mut bg_entries = vec![];
        for (binding, (binding_type, idx)) in asset_builder.elements.iter().enumerate() {
            match binding_type {
                RenderBindingType::Buffer => {
                    let buffer = gpu_buffers
                        .get(asset_builder.buffers[*idx as usize].unwrap())
                        .unwrap();
                    bg_entries.push(BindGroupBuilder::buffer(binding as u32, &buffer.buffer));
                }
                RenderBindingType::TextureView
                | RenderBindingType::TextureArrayView
                | RenderBindingType::StorageTexture => {
                    let texture = gpu_textures
                        .get(asset_builder.textures[*idx as usize].unwrap())
                        .unwrap();
                    bg_entries.push(BindGroupBuilder::texture_view(
                        binding as u32,
                        &texture.texture
                    ));
                }
                RenderBindingType::TextureSampler => {
                    let texture = gpu_textures
                        .get(asset_builder.textures[*idx as usize].unwrap())
                        .unwrap();
                    bg_entries.push(BindGroupBuilder::texture_sampler(
                        binding as u32,
                        &texture.texture
                    ));
                }
            }
        }
        let Ok(bind_group) =
            BindGroupBuilder::build(asset.label(), &render_instance, &layout_built, &bg_entries)
        else {
            trace!(
                "Not all resources for render binding {} are ready to create the bind group, retrying...",
                asset.label()
            );
            return Err(PrepareAssetError::RetryNextUpdate(asset));
        };

        // Return the gpu asset
        debug!("Prepared render binding {} GPU resources.", asset.label());
        Ok(GpuRenderBinding {
            _phantom: PhantomData,
            bind_group,
            layout
        })
    }
}
