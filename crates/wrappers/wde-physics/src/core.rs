use wde_logger::prelude::*;

use bevy::prelude::*;
use rapier3d::prelude::{
    ColliderHandle, Group, PhysicsWorld as RapierWorld, RigidBodyBuilder, RigidBodyHandle
};
use std::collections::HashMap;

use crate::{
    colliders::Collider,
    raycasting::{Ray, RayCastConfig}
};

/// The main physics world resource.
/// This resource manages the entire physics simulation, including all colliders, rigid bodies, and spatial queries. It provides methods for raycasting and automatically syncs with entities that have a [`Collider`] components.
#[derive(Resource, Default)]
pub struct PhysicsWorld {
    /// Rapier world. Never stepped, only its collision detection is run to keep scene queries up to date.
    rapier: RapierWorld,

    /// Mapping from Bevy entities to their Rapier collider handles
    entity_to_collider: HashMap<Entity, ColliderHandle>,
    /// Reverse mapping from Rapier collider handles to Bevy entities
    collider_to_entity: HashMap<ColliderHandle, Entity>,
    /// Mapping from Bevy entities to their Rapier rigid body handles
    entity_to_rigid_body: HashMap<Entity, RigidBodyHandle>,
    /// Reverse mapping from Rapier rigid body handles to Bevy entities
    rigid_body_to_entity: HashMap<RigidBodyHandle, Entity>
}
impl PhysicsWorld {
    /// Cast a ray in the physics world. See [`RayCastConfig`] for configuration options.
    /// See [crate] documentation for example usage.
    ///
    /// # Returns
    /// An optional tuple containing the hit entity and the time of impact (toi) if a hit occurred.
    pub fn cast_ray(&self, ray: &Ray, params: &RayCastConfig) -> Option<(Entity, f32)> {
        let (col_handle, toi) =
            self.rapier
                .cast_ray(&ray.0, params.max_toi, params.solid, params.filter)?;
        self.collider_to_entity
            .get(&col_handle)
            .map(|&entity| (entity, toi))
    }

    /// Membership groups of the collider attached to the entity, if any.
    pub fn collider_group(&self, entity: Entity) -> Option<Group> {
        let handle = self.entity_to_collider.get(&entity)?;
        self.rapier
            .colliders
            .get(*handle)
            .map(|collider| collider.collision_groups().memberships)
    }
}

/// System to handle changes in colliders, including additions, removals, and updates.
pub(crate) fn handle_changes(
    mut phworld: ResMut<PhysicsWorld>,
    colliders: Query<(Entity, &GlobalTransform, &Collider)>,
    new_collider: Query<Entity, Added<Collider>>,
    updated_collider: Query<Entity, Changed<Collider>>,
    updated_transform: Query<Entity, (Changed<GlobalTransform>, With<Collider>)>,
    mut removed_collider: RemovedComponents<Collider>
) {
    let phworld = &mut *phworld;

    // Add new colliders to the physics world
    {
        let _span = debug_span!("handle_new_colliders").entered();
        for entity in new_collider.iter().chain(updated_collider.iter()) {
            // Check if the collider already exists, and skip if it does
            if phworld.entity_to_collider.contains_key(&entity) {
                continue;
            }

            // Get the collider component and transform
            let (entity, transform, collider) = match colliders.get(entity) {
                Ok(data) => data,
                Err(_) => continue
            };

            // Insert a fixed rigid body holding the Rapier collider
            let rb_handle = phworld.rapier.insert_body(
                RigidBodyBuilder::fixed()
                    .translation(transform.translation())
                    .build()
            );
            let col = collider.data.read().unwrap().build();
            let col_handle = phworld.rapier.insert_collider(col, Some(rb_handle));

            phworld.entity_to_collider.insert(entity, col_handle);
            phworld.collider_to_entity.insert(col_handle, entity);
            phworld.entity_to_rigid_body.insert(entity, rb_handle);
            phworld.rigid_body_to_entity.insert(rb_handle, entity);
            trace!("Added the collider and rigid body of entity {entity}.");
        }
    }

    // Handle updated colliders
    {
        let _span = debug_span!("handle_updated_colliders").entered();
        for entity in updated_collider.iter() {
            if let Some(&col_handle) = phworld.entity_to_collider.get(&entity) {
                // Get the updated collider component
                let (entity, _, collider) = match colliders.get(entity) {
                    Ok(data) => data,
                    Err(_) => continue
                };

                // Replace the old collider, keeping its rigid body
                phworld.rapier.remove_collider(col_handle);
                phworld.collider_to_entity.remove(&col_handle);
                let col = collider.data.read().unwrap().build();
                let rb_handle = phworld.entity_to_rigid_body[&entity];
                let new_col_handle = phworld.rapier.insert_collider(col, Some(rb_handle));

                // Update the mappings
                phworld.entity_to_collider.insert(entity, new_col_handle);
                phworld.collider_to_entity.insert(new_col_handle, entity);
            }
        }
    }

    // Handle updated transforms
    {
        let _span = debug_span!("handle_updated_transforms").entered();
        for entity in updated_transform.iter() {
            if let Some(&rb_handle) = phworld.entity_to_rigid_body.get(&entity)
                && let Ok((_, transform, _)) = colliders.get(entity)
                && let Some(rigid_body) = phworld.rapier.bodies.get_mut(rb_handle)
            {
                rigid_body.set_translation(transform.translation(), true);
            }
        }
    }

    // Handle removed colliders
    let mut removed = false;
    {
        let _span = debug_span!("handle_removed_colliders").entered();
        for entity in removed_collider.read() {
            if let Some(col_handle) = phworld.entity_to_collider.remove(&entity) {
                phworld.collider_to_entity.remove(&col_handle);
                if let Some(rb_handle) = phworld.entity_to_rigid_body.remove(&entity) {
                    phworld.rigid_body_to_entity.remove(&rb_handle);
                    phworld.rapier.remove_body(rb_handle);
                }
                removed = true;
                trace!("Removed the collider and rigid body of entity {entity}.");
            }
        }
    }

    // Refresh the scene queries if there were any changes
    if !new_collider.is_empty()
        || !updated_collider.is_empty()
        || !updated_transform.is_empty()
        || removed
    {
        let _span = debug_span!("handle_changes_pipeline").entered();
        phworld.rapier.detect_collisions(&(), &());
    }
}
