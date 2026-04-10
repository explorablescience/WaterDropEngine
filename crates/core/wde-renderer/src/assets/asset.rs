use wde_logger::prelude::*;

use bevy::{
    app::{App, Plugin},
    ecs::{
        system::{StaticSystemParam, SystemParam, SystemParamItem, SystemState},
        world
    },
    platform::collections::{HashMap, HashSet},
    prelude::*
};
use thiserror::Error;

use crate::core::{Extract, MainWorld, Render, RenderApp, RenderSet};

#[derive(Debug, Error)]
pub enum PrepareAssetError<E: Send + Sync + 'static> {
    #[error("Failed to prepare asset. Retry next frame: {0}.")]
    RetryNextUpdate(E),
    #[error("Fatal error preparing asset: {0}.")]
    Fatal(String)
}
/// Trait that describes a GPU asset extracted from a CPU asset that implements the [bevy::prelude::Asset] trait.
/// The GPU asset is prepared from the CPU asset using the render-world system params, and can fail with a retry or fatal error.
pub trait RenderAsset: Send + Sync + 'static + Sized {
    type SourceAsset: Asset + Clone;
    type Params: SystemParam;

    /// Prepare the GPU asset from the CPU source [bevy::prelude::Asset] using the render-world system params.
    fn prepare(
        asset: Self::SourceAsset,
        params: &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>>;
    fn label(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// Stores all assets of a given GPU [RenderAsset] type, indexed by the ID of their source CPU asset.
#[derive(Resource)]
pub struct RenderAssets<A: RenderAsset>(HashMap<AssetId<A::SourceAsset>, A>);
impl<A: RenderAsset> Default for RenderAssets<A> {
    fn default() -> Self {
        Self(Default::default())
    }
}
impl<A: RenderAsset> RenderAssets<A> {
    pub fn get(&self, id: impl Into<AssetId<A::SourceAsset>>) -> Option<&A> {
        self.0.get(&id.into())
    }
    pub fn get_mut(&mut self, id: impl Into<AssetId<A::SourceAsset>>) -> Option<&mut A> {
        self.0.get_mut(&id.into())
    }
    pub fn insert(&mut self, id: impl Into<AssetId<A::SourceAsset>>, value: A) -> Option<A> {
        self.0.insert(id.into(), value)
    }
    pub fn remove(&mut self, id: impl Into<AssetId<A::SourceAsset>>) -> Option<A> {
        self.0.remove(&id.into())
    }
    pub fn iter(&self) -> impl Iterator<Item = (&AssetId<A::SourceAsset>, &A)> {
        self.0.iter()
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&AssetId<A::SourceAsset>, &mut A)> {
        self.0.iter_mut()
    }
}

/// Plugin that adds the systems and resources to extract, prepare and store a given type of GPU [RenderAsset] from their source CPU [bevy::prelude::Asset].
/// To use, simply add `RenderAssetsPlugin::<YourGpuAssetType>::default()` to your app, and make sure to implement the [RenderAsset] trait for your GPU asset type.
pub struct RenderAssetsPlugin<A: RenderAsset> {
    _phantom: std::marker::PhantomData<fn() -> A>
}
impl<A: RenderAsset> Default for RenderAssetsPlugin<A> {
    fn default() -> Self {
        Self {
            _phantom: Default::default()
        }
    }
}
impl<A: RenderAsset> Plugin for RenderAssetsPlugin<A> {
    fn build(&self, app: &mut App) {
        // Create the cached for extracting assets from the main world
        app.init_resource::<CachedExtractAssetsState<A>>();

        // Add the extract system to the renderer app
        let renderer_app = app.get_sub_app_mut(RenderApp).unwrap();
        renderer_app
            .init_resource::<PrepareNextFrameAssets<A>>()
            .init_resource::<ExtractedAssets<A>>()
            .init_resource::<RenderAssets<A>>()
            .add_systems(Extract, extract_render_assets::<A>);

        // Add the prepare system to the renderer app
        renderer_app.add_systems(Render, prepare_assets::<A>.in_set(RenderSet::Prepare));
    }
}

/// Stores the list of assets extracted from the main world AssetServer for the current frame, with their IDs and added/removed status.
#[allow(clippy::type_complexity)]
#[derive(Resource)]
struct CachedExtractAssetsState<A: RenderAsset> {
    state: SystemState<(
        MessageReader<'static, 'static, AssetEvent<A::SourceAsset>>,
        ResMut<'static, Assets<A::SourceAsset>>
    )>
}
impl<A: RenderAsset> FromWorld for CachedExtractAssetsState<A> {
    fn from_world(world: &mut world::World) -> Self {
        Self {
            state: SystemState::new(world)
        }
    }
}

/// Resource that stores the assets that failed to prepare in the previous frame and should be retried in the next frame.
#[derive(Resource)]
struct PrepareNextFrameAssets<A: RenderAsset> {
    assets: Vec<(AssetId<A::SourceAsset>, A::SourceAsset)>
}
impl<A: RenderAsset> Default for PrepareNextFrameAssets<A> {
    fn default() -> Self {
        Self {
            assets: Default::default()
        }
    }
}

/// Resource that stores the extracted assets from the main world AssetServer for the current frame, with their IDs and added/removed status.
#[derive(Resource)]
struct ExtractedAssets<A: RenderAsset> {
    /// List of IDs of the assets added this frame.
    pub added: HashSet<AssetId<A::SourceAsset>>,
    /// List of IDs of the assets removed this frame.
    pub removed: HashSet<AssetId<A::SourceAsset>>,
    /// The pair (id, CPU asset) of the added assets extracted this frame.
    pub extracted: Vec<(AssetId<A::SourceAsset>, A::SourceAsset)>
}
impl<A: RenderAsset> Default for ExtractedAssets<A> {
    fn default() -> Self {
        Self {
            extracted: Default::default(),
            removed: Default::default(),
            added: Default::default()
        }
    }
}

/// Extract the modified assets instructions from the main world AssetServer and load them to the renderer AssetServer.
fn extract_render_assets<A: RenderAsset>(
    mut commands: Commands,
    mut main_world: ResMut<MainWorld>
) {
    main_world.resource_scope(
        |main_world, mut cached_state: Mut<CachedExtractAssetsState<A>>| {
            let (mut events, mut assets) = cached_state.state.get_mut(main_world);
            let mut changed_assets: HashSet<AssetId<<A as RenderAsset>::SourceAsset>> =
                HashSet::default();
            let mut removed = HashSet::default();

            // Read all asset events and track the changed assets by their ID
            for event in events.read() {
                match event {
                    AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                        changed_assets.insert(*id);
                    }
                    AssetEvent::Unused { id } => {
                        changed_assets.remove(id);
                        removed.insert(*id);
                    }
                    AssetEvent::Removed { .. } => {}
                    AssetEvent::LoadedWithDependencies { .. } => {}
                }
            }

            // Add the changed assets to the extracted assets list
            let mut extracted_assets = Vec::new();
            let mut added = HashSet::new();
            for id in changed_assets.drain() {
                // Remove the asset from the main world AssetServer to avoid it being used by other systems while we prepare it for the GPU, and add it to the extracted assets list if it was present
                if let Some(asset) = assets.remove(id) {
                    extracted_assets.push((id, asset));
                    added.insert(id);
                }
            }
            commands.insert_resource(ExtractedAssets::<A> {
                extracted: extracted_assets,
                removed,
                added
            });

            // Apply all queued asset events
            cached_state.state.apply(main_world);
        }
    );
}

/// Load and unload the assets from the renderer based on the extracted assets.
fn prepare_assets<A: RenderAsset>(
    mut extracted_assets: ResMut<ExtractedAssets<A>>,
    mut render_assets: ResMut<RenderAssets<A>>,
    mut prepare_next_frame: ResMut<PrepareNextFrameAssets<A>>,
    params: StaticSystemParam<<A as RenderAsset>::Params>
) {
    let mut params = params.into_inner();
    let queued_assets = std::mem::take(&mut prepare_next_frame.assets);

    // Initialize the render assets from the previous frame that have not been finalized yet
    for (id, extracted_asset) in queued_assets {
        // Skip previous frame's assets removed or updated
        if extracted_assets.removed.contains(&id) || extracted_assets.added.contains(&id) {
            continue;
        }

        // Load the asset to the GPU from the CPU
        match A::prepare(extracted_asset, &mut params) {
            Ok(prepared_asset) => {
                // Add the asset to the render world
                render_assets.insert(id, prepared_asset);
            }
            Err(PrepareAssetError::RetryNextUpdate(extracted_asset)) => {
                // Try again next frame
                prepare_next_frame.assets.push((id, extracted_asset));
            }
            Err(PrepareAssetError::Fatal(error)) => {
                // Skip the asset
                error!("Fatal error preparing asset of id {}: {:?}.", id, error);
                extracted_assets.removed.insert(id);
            }
        }
    }

    // Remove assets
    for removed in extracted_assets.removed.drain() {
        let label = match render_assets.get(removed) {
            Some(asset) => asset.label(),
            None => "(asset not loaded)"
        };
        debug!(
            "Removing asset {} of type {}.",
            label,
            std::any::type_name::<A::SourceAsset>()
        );
        render_assets.remove(removed);
    }

    // Update changed assets
    for (id, extracted_asset) in extracted_assets.extracted.drain(..) {
        render_assets.remove(id);

        // Load the asset to the GPU from the CPU
        match A::prepare(extracted_asset, &mut params) {
            Ok(prepared_asset) => {
                // Add the asset to the render world
                render_assets.insert(id, prepared_asset);
            }
            Err(PrepareAssetError::RetryNextUpdate(extracted_asset)) => {
                // Try again next frame
                prepare_next_frame.assets.push((id, extracted_asset));
            }
            Err(PrepareAssetError::Fatal(error)) => {
                // Skip the asset
                error!("Fatal error preparing asset of id {}: {:?}", id, error);
            }
        }
    }
}
