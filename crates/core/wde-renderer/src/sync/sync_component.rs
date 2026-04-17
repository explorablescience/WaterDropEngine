use std::marker::PhantomData;

use bevy::{ecs::bundle::NoBundleEffect, prelude::*};

use crate::sync::{EntityRecord, PendingSyncEntity, SyncToRenderWorld};

/// Plugin that registers a component for automatic sync to the render world.
///
/// If an entity ([MainEntity](crate::sync::MainEntity)) has a component that implements [`SyncComponent`] it will automatically have the [`SyncToRenderWorld`] component added, which will trigger the synchronization of that entity to the render world ([RenderEntity](crate::sync::RenderEntity))).
/// See [`sync`](crate::sync) for more details.
pub struct SyncComponentPlugin<C, F = ()>(PhantomData<(C, F)>);
impl<C: SyncComponent<F>, F> Default for SyncComponentPlugin<C, F> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// Trait that links components from the main world with output components in the render world. It is used by [`SyncComponentPlugin`].
pub trait SyncComponent<F = ()>: Component {
    /// Describes what components should be removed from the render world if the implementing component is removed.
    type Target: Bundle<Effect: NoBundleEffect>;
}
impl<C: SyncComponent<F>, F: Send + Sync + 'static> Plugin for SyncComponentPlugin<C, F> {
    fn build(&self, app: &mut App) {
        // Register the SyncToRenderWorld component as required for this C component.
        app.register_required_components::<C, SyncToRenderWorld>();

        app.world_mut()
            .register_component_hooks::<C>()
            .on_remove(|mut world, context| {
                let mut pending = world.resource_mut::<PendingSyncEntity>();
                pending.push(EntityRecord::ComponentRemoved(
                    context.entity,
                    |mut entity| {
                        entity.remove::<C::Target>();
                    }
                ));
            });
    }
}
