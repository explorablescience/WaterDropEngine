use wde_logger::prelude::*;

use crate::prelude::*;
use bevy::{
    ecs::system::{
        StaticSystemParam, SystemParam, SystemParamItem, SystemState,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use std::{any::TypeId, collections::HashMap};

#[derive(Message)]
pub(crate) struct RenderDataRecreated(pub TypeId);

// ============= RENDER DATA PLUGIN REGISTER =============
/// A plugin to register a render data type.
///
/// This should be added to the app to initialize the [`RenderData`] and create its
/// corresponding [`GpuRenderData`]. If [`RenderData::recreate`] is implemented and returns
/// `Some(true)`, the data are recreated and dependencies are notified.
///
/// The GPU asset can then be retrieved using [`GpuRenderData`], and the binding index
/// specified in the description can be used to get the corresponding buffer or texture handle.
///
/// The render data type must implement [`RenderData`] and derive at least [`Asset`],
/// [`TypePath`] and [`Clone`].
pub struct RenderDataRegisterPlugin<R: RenderData>(std::marker::PhantomData<R>);
impl<R: RenderData> Default for RenderDataRegisterPlugin<R> {
    fn default() -> Self {
        RenderDataRegisterPlugin(std::marker::PhantomData)
    }
}
impl<R: RenderData + TypePath + Sync + Send + Clone + Asset> Plugin
    for RenderDataRegisterPlugin<R>
{
    fn build(&self, app: &mut App) {
        app.add_message::<RenderDataRecreated>();

        // Register asset
        app.init_asset::<RenderDataHolder<R>>();

        // Register render asset
        app.add_plugins(RenderAssetsPlugin::<GpuRenderData<R>>::default());
    }

    fn finish(&self, app: &mut App) {
        // Build the render data, add it to the asset server, and insert the handle as a resource for retrieval in the gpu creation stage
        let builder = {
            let mut builder = RenderDataBuilder::default();
            let mut state: SystemState<R::Params> = SystemState::new(app.world_mut());
            let mut params = state.get_mut(app.world_mut());
            R::describe(&mut params, &mut builder);
            builder
        };
        let handle = app
            .world_mut()
            .resource::<AssetServer>()
            .add(RenderDataHolder::<R> {
                _phantom: std::marker::PhantomData,
                builder
            });
        app.world_mut()
            .commands()
            .insert_resource(RenderDataHolderHandle(handle));

        // Add the recreate system if necessary
        let recreate = {
            let mut state: SystemState<R::Params> = SystemState::new(app.world_mut());
            let params = state.get_mut(app.world_mut());
            R::recreate(&params).is_some()
        };
        if recreate {
            app.add_systems(Update, recreate_system::<R>);
        }
    }
}

fn recreate_system<R: RenderData + TypePath + Sync + Send + 'static>(
    mut params: StaticSystemParam<R::Params>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut recreate_version: MessageWriter<RenderDataRecreated>
) {
    if let Some(true) = R::recreate(&params) {
        debug!("Recreating render data {}.", std::any::type_name::<R>());

        // Create new render data and update the asset
        let builder = {
            let mut builder = RenderDataBuilder::default();
            R::describe(&mut params, &mut builder);
            builder
        };
        let new_handle = asset_server.add(RenderDataHolder::<R> {
            _phantom: std::marker::PhantomData,
            builder
        });
        commands.insert_resource(RenderDataHolderHandle(new_handle));

        // Add that the render data have been recreated for this type
        recreate_version.write(RenderDataRecreated(TypeId::of::<R>()));
    }
}

// ============= RENDER DATA AND BUILDER METHODS =============
/// A builder for render data. This is used to describe the buffers and textures that should be created on the gpu for a render data.
/// After each data have been described, buffers and textures will be created on the gpu according to the description, and can be retrieved on the GPU side (see [`GpuRenderData`](GpuRenderData)).
#[derive(Default, Clone)]
pub struct RenderDataBuilder {
    binding_to_index: HashMap<u32, (usize, usize)>, // binding index, (binding type (0 for buffer, 1 for texture), idx in array)
    buffers: Vec<Buffer>,
    textures: Vec<Texture>
}
impl RenderDataBuilder {
    /// Adds a buffer to the render data. The binding index is used to retrieve the buffer later when using this render data.
    pub fn add_buffer(&mut self, binding: u32, buffer: Buffer) -> &mut Self {
        self.buffers.push(buffer);
        self.binding_to_index
            .insert(binding, (0, self.buffers.len() - 1));
        self
    }
    /// Adds a texture to the render data. The binding index is used to retrieve the texture later when using this render data.
    pub fn add_texture(&mut self, binding: u32, texture: Texture) -> &mut Self {
        self.textures.push(texture);
        self.binding_to_index
            .insert(binding, (1, self.textures.len() - 1));
        self
    }
}

/// A trait for describing render data.
///
/// This trait is used to declare buffers and textures that should be created on the GPU.
/// The struct implementing this trait should usually derive [`Asset`], [`TypePath`] and
/// [`Clone`] so it can be registered through [`RenderDataRegisterPlugin`].
///
/// The prepared GPU side can then be retrieved in render systems through [`GpuRenderData`],
/// using the binding indices specified in [`RenderData::describe`].
pub trait RenderData {
    type Params: SystemParam;

    /// Describes the render data by adding buffers and textures to the builder.
    ///
    /// This is called during initialization, and when [`recreate`](Self::recreate)
    /// returns `Some(true)`.
    fn describe(params: &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder);
    /// Whether the gpu asset should be recreated. This is called every frame. The default is to never recreate (None).
    fn recreate(_params: &SystemParamItem<Self::Params>) -> Option<bool> {
        None
    }
}

// ============= RENDER DATA BUILDER TEMPORARY STORAGE =============
#[allow(unused)]
#[derive(Resource)]
struct RenderDataHolderHandle<R: RenderData + TypePath + Sync + Send>(Handle<RenderDataHolder<R>>);

#[derive(Asset, Clone, TypePath)]
pub struct RenderDataHolder<R: RenderData + TypePath + Sync + Send> {
    _phantom: std::marker::PhantomData<R>,
    builder: RenderDataBuilder
}

// ============= RENDER DATA RENDER ASSET GPU CREATION =============
/// The gpu asset that is created from the render data holder. This is the asset that is actually used in the render pass.
pub type SRenderData<T> = SRes<RenderAssets<GpuRenderData<T>>>;
/// The gpu asset that is created from the render data holder. This is the asset that is actually used in the render pass.
pub type SRenderDataMut<T> = SResMut<RenderAssets<GpuRenderData<T>>>;

/// Alias for a `Res<RenderAssets<GpuRenderData<M>>>`.
pub type ResRenderData<'w, M> = Res<'w, RenderAssets<GpuRenderData<M>>>;
/// Alias for a `ResMut<RenderAssets<GpuRenderData<M>>>`.
pub type ResMutRenderData<'w, M> = ResMut<'w, RenderAssets<GpuRenderData<M>>>;

/// The gpu asset that is created from the render data holder. This is the asset that is actually used in the render pass.
/// It contains the gpu buffers and textures that are created according to the description in the [`RenderData`](RenderData) implementation, and can be retrieved using the binding index specified in the description.
pub struct GpuRenderData<R: RenderData> {
    _phantom: std::marker::PhantomData<R>,
    builder: RenderDataBuilder,
    buffers: Vec<Handle<Buffer>>,
    textures: Vec<Handle<Texture>>
}
impl<R: RenderData> GpuRenderData<R> {
    /// Retrieves the buffer handle for the given binding index.
    pub fn get_buffer(&self, binding: u32) -> Option<Handle<Buffer>> {
        self.builder
            .binding_to_index
            .get(&binding)
            .and_then(|(ty, idx)| {
                debug_assert!(
                    *ty == 0,
                    "Tried to get buffer for binding {}, but it was a texture",
                    binding
                );
                self.buffers.get(*idx).cloned()
            })
    }
    /// Retrieves the texture handle for the given binding index.
    pub fn get_texture(&self, binding: u32) -> Option<Handle<Texture>> {
        self.builder
            .binding_to_index
            .get(&binding)
            .and_then(|(ty, idx)| {
                debug_assert!(
                    *ty == 1,
                    "Tried to get texture for binding {}, but it was a buffer",
                    binding
                );
                self.textures.get(*idx).cloned()
            })
    }
}
impl<R: RenderData + Clone + Asset> RenderAsset for GpuRenderData<R> {
    type SourceAsset = RenderDataHolder<R>;
    type Params = SRes<AssetServer>;

    fn prepare(
        asset: Self::SourceAsset,
        asset_server: &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        debug!(
            "Preparing render binding {} GPU resources.",
            std::any::type_name::<R>()
        );
        let mut buffers = Vec::new();
        let mut textures = Vec::new();
        let mut updated_builder = RenderDataBuilder::default();
        for (binding, (ty, idx)) in asset.builder.binding_to_index.iter() {
            match ty {
                0 => {
                    // buffer
                    let mut buffer = asset.builder.buffers.get(*idx).unwrap().clone();
                    let handle = asset_server.add(buffer.clone());
                    buffers.push(handle);

                    buffer.content = None; // free cpu side content as it's now on gpu, and we won't need it anymore
                    updated_builder.add_buffer(*binding, buffer);
                }
                1 => {
                    // texture
                    let mut texture = asset.builder.textures.get(*idx).unwrap().clone();
                    let handle = asset_server.add(texture.clone());
                    textures.push(handle);

                    texture.data = vec![]; // free cpu side content as it's now on gpu, and we won't need it anymore
                    updated_builder.add_texture(*binding, texture);
                }
                _ => unreachable!()
            }
        }
        Ok(GpuRenderData {
            _phantom: std::marker::PhantomData,
            builder: updated_builder,
            buffers,
            textures
        })
    }
}
