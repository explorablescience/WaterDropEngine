use bevy::prelude::*;

use crate::sync::{MainEntity, RenderEntity};

/// Marker component that indicates that its entity needs to be synchronized to the render world.
/// 
/// It is added automatically to entities that have components that implement [`SyncComponent`](crate::sync::sync_component::SyncComponent) (see [`SyncComponentPlugin`](crate::sync::sync_component::SyncComponentPlugin)), and can be added manually to entities that need to be synchronized without any specific component requirements.
/// See [`sync`](crate::sync) for more details.
#[derive(Component, Copy, Clone, Debug, Default, Reflect)]
#[reflect(Component, Default, Clone)]
#[component(storage = "SparseSet")]
pub struct SyncToRenderWorld;

pub(crate) struct SyncWorldPlugin;
impl Plugin for SyncWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingSyncEntity>()
            .add_observer(
                |add: On<Add, SyncToRenderWorld>, mut pending: ResMut<PendingSyncEntity>| {
                    pending.push(EntityRecord::Added(add.entity)); // Push the added main world entity to the pending sync list
                },
            )
            .add_observer(
                |remove: On<Remove, SyncToRenderWorld>,
                 mut pending: ResMut<PendingSyncEntity>,
                 query: Query<&RenderEntity>| {
                    if let Ok(e) = query.get(remove.entity) {
                        pending.push(EntityRecord::Removed(*e)); // Push the removed main world entity to the pending sync list
                    };
                },
            );
    }
}

/// A record enum to what entities with [`SyncToRenderWorld`] have been added or removed.
#[derive(Debug)]
pub(crate) enum EntityRecord {
    /// Entity spawned on the main world.
    Added(Entity),
    /// Entity despawned on the main world.
    Removed(RenderEntity),
    /// Component removed on an entity in the main world.
    ComponentRemoved(Entity, fn(EntityWorldMut<'_>)),
}

// Entity Record in Main World pending to Sync to the Render World.
#[derive(Resource, Default, Deref, DerefMut)]
pub(crate) struct PendingSyncEntity {
    records: Vec<EntityRecord>,
}

pub(crate) fn sync_worlds(main_world: &mut World, render_world: &mut World) {
    main_world.resource_scope(|world, mut pending: Mut<PendingSyncEntity>| {
        for record in pending.drain(..) {
            match record {
                EntityRecord::Added(e) => {
                    if let Ok(mut main_entity) = world.get_entity_mut(e) {
                        match main_entity.entry::<RenderEntity>() {
                            bevy::ecs::world::ComponentEntry::Occupied(_) => {
                                panic!("Attempting to synchronize an entity that has already been synchronized!");
                            }
                            bevy::ecs::world::ComponentEntry::Vacant(entry) => {
                                let id = render_world.spawn(MainEntity(e)).id();

                                entry.insert(RenderEntity(id));
                            }
                        };
                    }
                }
                EntityRecord::Removed(render_entity) => {
                    if let Ok(ec) = render_world.get_entity_mut(render_entity.id()) {
                        ec.despawn();
                    };
                }
                EntityRecord::ComponentRemoved(main_entity, removal_function) => {
                    let Some(render_entity) = world.get::<RenderEntity>(main_entity) else {
                        continue;
                    };
                    if let Ok(render_world_entity) = render_world.get_entity_mut(render_entity.id()) {
                        removal_function(render_world_entity);
                    }
                },
            }
        }
    });
}
