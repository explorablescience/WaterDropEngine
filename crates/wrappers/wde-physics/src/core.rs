use wde_logger::prelude::*;

use bevy::prelude::*;
use rapier3d::prelude::*;
use std::{collections::HashMap, sync::RwLock};

use crate::{
    colliders::Collider,
    raycasting::{Ray, RayCastConfig}
};

/// Internal handler for Rapier physics components.
#[derive(Default)]
struct RapierHandler {
    /// Set of all colliders in the physics world
    collider_set: RwLock<ColliderSet>,
    /// Set of all rigid bodies in the physics world
    rigid_body_set: RwLock<RigidBodySet>,

    /// Spatial query acceleration structure for raycasting and intersection tests.
    /// Automatically updated when colliders are added, modified, or removed.
    query_pipeline: RwLock<QueryPipeline>,

    /// Manages simulation islands for performance optimization
    island_manager: RwLock<IslandManager>,
    /// Manages impulse-based joints between rigid bodies
    impulse_joint_set: RwLock<ImpulseJointSet>,
    /// Manages multibody joints (reduced coordinates articulations)
    multibody_joint_set: RwLock<MultibodyJointSet>
}

/// The main physics world resource.
/// This resource manages the entire physics simulation, including all colliders, rigid bodies, and spatial queries. It provides methods for raycasting and automatically syncs with entities that have a [`Collider`] components.
#[derive(Resource, Default)]
pub struct PhysicsWorld {
    /// Internal Rapier physics handler
    rapier: RapierHandler,

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
        let collider_set = self.rapier.collider_set.read().unwrap();
        let rigid_body_set = self.rapier.rigid_body_set.read().unwrap();
        let query_pipeline = self.rapier.query_pipeline.read().unwrap();

        if let Some((col_handle, toi)) = query_pipeline.cast_ray(
            &rigid_body_set,
            &collider_set,
            &ray.0,
            params.max_toi,
            params.solid,
            params.filter
        ) && let Some(&entity) = self.collider_to_entity.get(&col_handle)
        {
            return Some((entity, toi));
        }
        None
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
            // Build the Rapier collider and insert it into the collider set
            let col = collider.data.read().unwrap().build();
            let col_handle = phworld.rapier.collider_set.write().unwrap().insert(col);

            // Create a rigid body for the collider and insert it into the rigid body set
            let rb = RigidBodyBuilder::fixed()
                .translation(vector![
                    transform.translation().x,
                    transform.translation().y,
                    transform.translation().z
                ])
                .build();
            let rb_handle = phworld.rapier.rigid_body_set.write().unwrap().insert(rb);

            // Associate the collider with its rigid body and store the mapping
            phworld.rapier.collider_set.write().unwrap().set_parent(
                col_handle,
                Some(rb_handle),
                &mut phworld.rapier.rigid_body_set.write().unwrap()
            );
            phworld.entity_to_collider.insert(entity, col_handle);
            phworld.collider_to_entity.insert(col_handle, entity);
            phworld.entity_to_rigid_body.insert(entity, rb_handle);
            phworld.rigid_body_to_entity.insert(rb_handle, entity);
            trace!(
                "Added collider and rigidbody for entity {:?} with collider {:?} and rigidbody {:?} handles",
                entity, col_handle, rb_handle
            );
        }
    }

    // Handle updated colliders
    {
        let _span = debug_span!("handle_updated_colliders").entered();
        for entity in updated_collider.iter() {
            if let Some(&col_handle) = phworld.entity_to_collider.get(&entity) {
                // Remove the old collider
                phworld.rapier.collider_set.write().unwrap().remove(
                    col_handle,
                    &mut phworld.rapier.island_manager.write().unwrap(),
                    &mut phworld.rapier.rigid_body_set.write().unwrap(),
                    true
                );

                // Get the updated collider component
                let (entity, _, collider) = match colliders.get(entity) {
                    Ok(data) => data,
                    Err(_) => continue
                };
                // Build the new Rapier collider and insert it into the collider set
                let col = collider.data.read().unwrap().build();
                let new_col_handle = phworld.rapier.collider_set.write().unwrap().insert(col);

                // Re-associate the new collider with its rigid body
                let rb_handle = phworld.entity_to_rigid_body[&entity];
                phworld.rapier.collider_set.write().unwrap().set_parent(
                    new_col_handle,
                    Some(rb_handle),
                    &mut phworld.rapier.rigid_body_set.write().unwrap()
                );

                // Update the mappings
                phworld.entity_to_collider.insert(entity, new_col_handle);
                phworld.collider_to_entity.insert(new_col_handle, entity);
            }
        }
    }

    // Handle updated transforms
    {
        let _span = trace_span!("handle_updated_transforms").entered();
        for entity in updated_transform.iter() {
            if let Some(&rb_handle) = phworld.entity_to_rigid_body.get(&entity) {
                // Get the updated transform
                let (_, transform, _) = match colliders.get(entity) {
                    Ok(data) => data,
                    Err(_) => continue
                };
                // Update the rigid body's position
                if let Some(rigid_body) = phworld
                    .rapier
                    .rigid_body_set
                    .write()
                    .unwrap()
                    .get_mut(rb_handle)
                {
                    rigid_body.set_translation(
                        vector![
                            transform.translation().x,
                            transform.translation().y,
                            transform.translation().z
                        ],
                        true
                    );
                }
            }
        }
        drop(_span);
    }

    // Generate list of entities to remove colliders for
    let mut removed_collider_entities = Vec::new();
    removed_collider.read().for_each(|entity| {
        removed_collider_entities.push(entity);
    });

    // Update the query pipeline if there were any changes
    if !new_collider.is_empty()
        || !updated_collider.is_empty()
        || !updated_transform.is_empty()
        || !removed_collider_entities.is_empty()
    {
        let _span = trace_span!("handle_changes_pipeline").entered();
        let mut modified_colliders = Vec::new();
        for entity in new_collider
            .iter()
            .chain(updated_collider.iter())
            .chain(updated_transform.iter())
        {
            if let Some(&col_handle) = phworld.entity_to_collider.get(&entity) {
                modified_colliders.push(col_handle);
            }
        }
        let mut removed_colliders = Vec::new();
        removed_collider_entities.iter().for_each(|&entity| {
            if let Some(&col_handle) = phworld.entity_to_collider.get(&entity) {
                removed_colliders.push(col_handle);
            }
        });

        // Update the query pipeline incrementally
        // This is more efficient than rebuilding it from scratch
        let colliders = &phworld.rapier.collider_set.read().unwrap();
        phworld
            .rapier
            .query_pipeline
            .write()
            .unwrap()
            .update_incremental(colliders, &modified_colliders, &removed_colliders, true);
    }

    // Handle removed colliders
    {
        let _span = debug_span!("handle_removed_colliders").entered();
        removed_collider_entities.iter().for_each(|&entity| {
            if let Some(&col_handle) = phworld.entity_to_collider.get(&entity) {
                // Remove the collider from the collider set
                phworld.rapier.rigid_body_set.write().unwrap().remove(
                    phworld.entity_to_rigid_body[&entity],
                    &mut phworld.rapier.island_manager.write().unwrap(),
                    &mut phworld.rapier.collider_set.write().unwrap(),
                    &mut phworld.rapier.impulse_joint_set.write().unwrap(),
                    &mut phworld.rapier.multibody_joint_set.write().unwrap(),
                    true
                );

                // Remove the mappings
                phworld.entity_to_collider.remove(&entity);
                phworld.collider_to_entity.remove(&col_handle);
                debug!(
                    "Removed collider and rigidbody for entity {:?} with handle {:?}",
                    entity, col_handle
                );
            }
        });
    }
}
