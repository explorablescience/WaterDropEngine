use bevy::prelude::*;

use crate::{
    assets::{RenderAsset, RenderAssetsPlugin},
    core::RenderApp
};

/// Describes a render pipeline asset (used by [`RenderPipelinePluginRegister`]) which will be responsible of storing the render pipeline instance.
/// Most of the time, you will use it as a type alias for the `SourceAsset` of your render pipeline.
/// The render pipeline should derive the [`RenderAsset`], [`TypePath`], [`Default`], [`Clone`] and [`Debug`] traits, and implement the `prepare` function to create the render pipeline instance from the asset data.
/// See [crate::passes] for more details and examples.
#[derive(Default, Asset, Clone, TypePath, Debug)]
pub struct RenderPipelineAsset<P: RenderAsset + TypePath>(pub P);
impl<P: RenderAsset + TypePath> std::fmt::Display for RenderPipelineAsset<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RenderPipelineAsset<{}>", std::any::type_name::<P>())
    }
}

/// Component to hold a render pipeline asset handle, preventing it from being dropped and unloaded.
#[allow(unused)]
#[derive(Component)]
struct RenderPipelineAssetHolder<P: RenderAsset + TypePath>(pub Handle<RenderPipelineAsset<P>>);

/// A plugin for registering a render pipeline asset and its preparation logic.
/// It will also spawn an entity holding the render pipeline asset handle, preventing it from being dropped and unloaded.
///
/// To use it, simply add it to your app with the render pipeline asset type as generic parameter:
/// ```rust
/// app.add_plugins(RenderPipelinePluginRegister::<MyRenderPipelineAsset>::default());
/// ```
/// The render pipeline should derive the [`RenderAsset`], [`TypePath`], [`Default`] and [`Clone`] traits.
/// It should use the [`RenderPipelineAsset`] as its `SourceAsset` type.
///
/// See [crate::passes] for more details and examples.
pub struct RenderPipelinePluginRegister<P: RenderAsset + TypePath + Default> {
    _phantom: std::marker::PhantomData<P>
}
impl<P: RenderAsset + TypePath + Default> Default for RenderPipelinePluginRegister<P> {
    fn default() -> Self {
        Self {
            _phantom: std::marker::PhantomData
        }
    }
}
impl<P: RenderAsset + TypePath + Default> Plugin for RenderPipelinePluginRegister<P> {
    fn build(&self, app: &mut App) {
        // Add the render pipeline asset and its preparation plugin
        app.init_asset::<RenderPipelineAsset<P>>()
            .add_plugins(RenderAssetsPlugin::<P>::default());
    }

    fn finish(&self, app: &mut App) {
        // Create the render pipeline asset
        let pipeline = app
            .world_mut()
            .get_resource::<AssetServer>()
            .unwrap()
            .add(RenderPipelineAsset::<P>::default());

        // Spawn an entity holding the render pipeline asset handle, avoiding it to be dropped and unloaded
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .world_mut()
            .spawn(RenderPipelineAssetHolder(pipeline));
    }
}
