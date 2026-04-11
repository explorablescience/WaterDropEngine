use std::marker::PhantomData;

use bevy::{
    ecs::{
        bundle::NoBundleEffect,
        query::{QueryFilter, ReadOnlyQueryData},
    },
    prelude::*,
};

use crate::{
    core::{Extract, ExtractWorld, RenderApp},
    sync::{RenderEntity, SyncComponent, SyncComponentPlugin},
};

pub use bevy::ecs::query::QueryItem;

/// Describes how a component gets extracted for rendering.
///
/// Therefore the component is transferred from the "app world" into the "render world" in the [`Extract`] step. This functionality is enabled by adding [`ExtractComponentPlugin`] with the component type.
///
/// The Out type is defined in [`SyncComponent`].
/// See [sync](crate::sync) for more details.
pub trait ExtractComponent<F = ()>: SyncComponent<F> {
    /// ECS [`ReadOnlyQueryData`] to fetch the components to extract.
    type QueryData: ReadOnlyQueryData;
    /// Filters the entities with additional constraints.
    type QueryFilter: QueryFilter;
    /// The output from extraction, i.e. [`ExtractComponent::extract_component`].
    type Out: Bundle<Effect: NoBundleEffect>;

    /// Defines how the component is transferred into the "render world".
    /// Returning `None` based on the queried item will remove the [`SyncComponent::Target`] from the entity in the render world.
    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out>;
}

/// This plugin extracts the components into the render world for synced entities.
/// To do so, it sets up the [`Extract`] step for the specified [`ExtractComponent`].
///
/// It also registers [`SyncComponentPlugin`] to ensure the extracted components are deleted if the main world components are removed.
/// See [sync](crate::sync) for more details.
pub struct ExtractComponentPlugin<C, F = ()>(PhantomData<fn() -> (C, F)>);
impl<C, F> Default for ExtractComponentPlugin<C, F> {
    fn default() -> Self {
        Self(PhantomData)
    }
}
impl<C: ExtractComponent<F>, F: 'static + Send + Sync> Plugin for ExtractComponentPlugin<C, F> {
    fn build(&self, app: &mut App) {
        app.add_plugins(SyncComponentPlugin::<C, F>::default());
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .add_systems(Extract, extract_components::<C, F>);
    }
}

/// This system extracts all components of the corresponding [`ExtractComponent`], for entities that are synced via [`crate::sync_world::SyncToRenderWorld`].
#[allow(clippy::type_complexity)]
fn extract_components<C: ExtractComponent<F>, F>(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: ExtractWorld<Query<(RenderEntity, C::QueryData), C::QueryFilter>>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, query_item) in &query {
        if let Some(component) = C::extract_component(query_item) {
            values.push((entity, component));
        } else {
            commands.entity(entity).remove::<C::Target>();
        }
    }
    *previous_len = values.len();
    commands.try_insert_batch(values);
}
