//! Extract the resources from the scene and load them to the GPU in the renderer.

use bevy::{app::{App, Plugin}, ecs::{schedule::ScheduleConfigs, system::{ScheduleSystem, StaticSystemParam, SystemParam, SystemParamItem, SystemState}, world}, platform::collections::{HashMap, HashSet}, prelude::*};
use thiserror::Error;

use crate::core::{Extract, MainWorld, Render, RenderApp, RenderSet};


#[derive(Debug, Error)]
#[allow(unused)]
/// Error that can occur when loading an asset to the GPU for rendering.
pub enum PrepareAssetError<E: Send + Sync + 'static> {
    #[error("Failed to prepare asset. Retry next frame: {0}.")]
    /// The asset failed to prepare and should be retried next update.
    RetryNextUpdate(E),
    #[error("Fatal error preparing asset: {0}.")]
    /// The asset failed to prepare and will not be retried.
    Fatal(String),
}

/// Trait for assets that can be loaded to the GPU for rendering.
pub trait RenderAsset: Send + Sync + 'static + Sized {
    type SourceAsset: Asset + Clone;
    type Param: SystemParam;

    /// Load the asset to the GPU from the CPU.
    fn prepare_asset(
        asset: Self::SourceAsset,
        param: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>>;

    /// Return the label of the asset.
    fn label(&self) -> &str;
}



#[derive(Resource)]
/// Stores the state of the cached extract assets system.
struct CachedExtractAssetsState<A: RenderAsset> {
    #[allow(clippy::type_complexity)]
    state: SystemState<(
        MessageReader<'static, 'static, AssetEvent<A::SourceAsset>>,
        ResMut<'static, Assets<A::SourceAsset>>
    )>
}
impl<A: RenderAsset> FromWorld for CachedExtractAssetsState<A> {
    fn from_world(world: &mut world::World) -> Self {
        Self { state: SystemState::new(world) }
    }
}


// Helper to allow specifying dependencies between render assets
pub trait RenderAssetDependency {
    fn register_system(render_app: &mut SubApp, system: ScheduleConfigs<ScheduleSystem>);
}
impl RenderAssetDependency for () {
    fn register_system(render_app: &mut SubApp, system: ScheduleConfigs<ScheduleSystem>) {
        render_app.add_systems(Render, system);
    }
}
impl<A: RenderAsset> RenderAssetDependency for A {
    fn register_system(render_app: &mut SubApp, system: ScheduleConfigs<ScheduleSystem>) {
        render_app.add_systems(Render, system.after(prepare_assets::<A>));
    }
}


/// Temporarily stores the extracted and removed assets of the current frame.
#[derive(Resource)]
struct ExtractedAssets<A: RenderAsset> {
    /// List of IDs of the assets added this frame.
    pub added: HashSet<AssetId<A::SourceAsset>>,
    /// List of IDs of the assets removed this frame.
    pub removed: HashSet<AssetId<A::SourceAsset>>,

    /// The pair (id, CPU asset) of the added assets extracted this frame.
    pub extracted: Vec<(AssetId<A::SourceAsset>, A::SourceAsset)>,
}
impl<A: RenderAsset> Default for ExtractedAssets<A> {
    fn default() -> Self {
        Self {
            extracted: Default::default(),
            removed: Default::default(),
            added: Default::default(),
        }
    }
}



#[derive(Resource)]
/// List of assets to prepare in the next frame that failed to prepare in the current frame.
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


/// Stores all GPU representations of the assets.
#[derive(Resource)]
pub struct RenderAssets<A: RenderAsset>(HashMap<AssetId<A::SourceAsset>, A>);
impl<A: RenderAsset> Default for RenderAssets<A> {
    fn default() -> Self {
        Self(Default::default())
    }
}
#[allow(unused)]
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

    pub fn iter(&self) -> impl Iterator<Item = (AssetId<A::SourceAsset>, &A)> {
        self.0.iter().map(|(k, v)| (*k, v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (AssetId<A::SourceAsset>, &mut A)> {
        self.0.iter_mut().map(|(k, v)| (*k, v))
    }
}





/// Plugin that adds the render assets system to the renderer app.
pub struct RenderAssetsPlugin<A: RenderAsset, AFTER: RenderAssetDependency + 'static = ()> {
    phantom: std::marker::PhantomData<fn() -> (A, AFTER)>
}
impl<A: RenderAsset, AFTER: RenderAssetDependency + 'static> Plugin for RenderAssetsPlugin<A, AFTER> {
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
        AFTER::register_system(
            renderer_app,
            prepare_assets::<A>.in_set(RenderSet::PrepareAssets),
        );
    }
}
impl<A: RenderAsset, AFTER: RenderAssetDependency + 'static> Default for RenderAssetsPlugin<A, AFTER> {
    fn default() -> Self {
        Self { phantom: Default::default() }
    }
}





/// Extract the modified assets instructions from the main world AssetServer and load them to the renderer AssetServer.
fn extract_render_assets<A: RenderAsset>(mut commands: Commands, mut main_world: ResMut<MainWorld>) {
    main_world.resource_scope(|main_world, mut cached_state: Mut<CachedExtractAssetsState<A>>| {
        let (mut events, mut assets) = cached_state.state.get_mut(main_world);

        let mut changed_assets: HashSet<AssetId<<A as RenderAsset>::SourceAsset>> = HashSet::default();
        let mut removed = HashSet::default();

        for event in events.read() {
            match event {
                AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                    // Add the asset to the render world
                    changed_assets.insert(*id);
                }
                AssetEvent::Removed { .. } => {}
                AssetEvent::Unused { id } => {
                    // Remove the asset from the render world
                    changed_assets.remove(id);
                    removed.insert(*id);
                }
                AssetEvent::LoadedWithDependencies { .. } => {}
            }
        }

        // Extract all changed assets to the render world
        let mut extracted_assets = Vec::new();
        let mut added = HashSet::new();
        for id in changed_assets.drain() {
            if let Some(asset) = assets.remove(id) {
                extracted_assets.push((id, asset));
                added.insert(id);
            }
        }

        // Update the resource with the new asset set
        commands.insert_resource(ExtractedAssets::<A> {
            extracted: extracted_assets,
            removed,
            added,
        });

        // Apply all queued asset events
        cached_state.state.apply(main_world);
    });
}

/// Load and unload the assets from the renderer based on the extracted assets.
fn prepare_assets<A: RenderAsset>(
    mut extracted_assets: ResMut<ExtractedAssets<A>>,
    mut render_assets: ResMut<RenderAssets<A>>,
    mut prepare_next_frame: ResMut<PrepareNextFrameAssets<A>>,
    param: StaticSystemParam<<A as RenderAsset>::Param>
) {
    let mut param = param.into_inner();
    let queued_assets = std::mem::take(&mut prepare_next_frame.assets);

    // Initialize the render assets from the previous frame that have not been finalized yet
    for (id, extracted_asset) in queued_assets {
        // Skip previous frame's assets removed or updated
        if extracted_assets.removed.contains(&id) || extracted_assets.added.contains(&id) {
            continue;
        }

        // Load the asset to the GPU from the CPU
        match A::prepare_asset(extracted_asset, &mut param) {
            Ok(prepared_asset) => {
                // Add the asset to the render world
                render_assets.insert(id, prepared_asset);
            }
            Err(PrepareAssetError::RetryNextUpdate(extracted_asset)) => {
                // Try again next frame
                prepare_next_frame.assets.push((id, extracted_asset));
            },
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
        debug!("Removing asset of type {} labeled {}.", std::any::type_name::<A::SourceAsset>(), label);
        render_assets.remove(removed);
    }

    // Update changed assets
    for (id, extracted_asset) in extracted_assets.extracted.drain(..) {
        render_assets.remove(id);

        // Load the asset to the GPU from the CPU
        match A::prepare_asset(extracted_asset, &mut param) {
            Ok(prepared_asset) => {
                // Add the asset to the render world
                render_assets.insert(id, prepared_asset);
            }
            Err(PrepareAssetError::RetryNextUpdate(extracted_asset)) => {
                // Try again next frame
                prepare_next_frame.assets.push((id, extracted_asset));
            },
            Err(PrepareAssetError::Fatal(error)) => {
                // Skip the asset
                error!("Fatal error preparing asset of id {}: {:?}.", id, error);
            }
        }
    }
}
