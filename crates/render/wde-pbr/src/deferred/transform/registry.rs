use std::collections::HashMap;
use wde_logger::prelude::*;

use bevy::prelude::*;
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::prelude::{PbrSsboTransform, SSBO_TRANSFORM_MAX_ENTITY};

/// Marker component to indicate that an entity has a PBR material and should be included in the SSBO transform updates.
#[derive(Component, Default)]
pub struct PbrSsboTransformMarker;

pub(crate) struct SsboTransformRegistryPlugin;
impl Plugin for SsboTransformRegistryPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<PbrSsboTransformRegistry>()
            .add_plugins(ExtractResourcePlugin::<PbrSsboTransformRegistry>::default())
            .add_systems(Update, set_dirty_transforms);
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .add_systems(Render, update_ssbo_transforms.in_set(RenderSet::Prepare));
    }
}

/// Resource to track the mapping between entities and their transform IDs in the SSBO, as well as the list of dirty transforms that need to be updated in the SSBO. This is used to update the [PbrSsboTransform] with the new transforms of the entities with a PBR material.
#[derive(Resource, Default, Clone)]
pub struct PbrSsboTransformRegistry {
    pub entity_to_transform: HashMap<Entity, u32>, // (entity, transform_id in the SSBO)
    pub available_transform_ids: Vec<u32>,         // List of available transform IDs in the SSBO
    pub next_transform_id: u32,                    // Next available transform ID in the SSBO
    pub dirty_transforms: Vec<(u32, TransformUniform)>, // List of dirty transforms that need to be updated in the SSBO
}
impl ExtractResource for PbrSsboTransformRegistry {
    type Source = Self;
    fn extract(source: &Self::Source) -> Self {
        Self {
            entity_to_transform: source.entity_to_transform.clone(),
            available_transform_ids: vec![], // Don't need it on the render thread
            next_transform_id: 0,            // Don't need it on the render thread
            dirty_transforms: source.dirty_transforms.clone(),
        }
    }
}
impl PbrSsboTransformRegistry {
    /// Register an entity with its transform in the registry, returning the assigned transform ID in the SSBO. If the entity is already registered, returns the existing transform ID.
    pub fn add_entity(&mut self, entity: Entity, transform: GlobalTransform) -> u32 {
        if let Some(&transform_id) = self.entity_to_transform.get(&entity) {
            transform_id
        } else {
            let transform_id = if let Some(free_id) = self.available_transform_ids.pop() {
                free_id
            } else if self.next_transform_id < SSBO_TRANSFORM_MAX_ENTITY as u32 {
                let id = self.next_transform_id;
                self.next_transform_id += 1;
                id
            } else {
                error!(
                    "PbrSsboTransformRegistry: Maximum number of transform IDs reached ({})",
                    SSBO_TRANSFORM_MAX_ENTITY
                );
                0
            };
            self.entity_to_transform.insert(entity, transform_id);
            self.dirty_transforms
                .push((transform_id, TransformUniform::new(&transform)));
            transform_id
        }
    }

    /// Get the transform ID of an entity in the registry, if it exists.
    pub fn get_transform_id(&self, entity: Entity) -> Option<u32> {
        self.entity_to_transform.get(&entity).copied()
    }

    /// Update the transform of an entity in the registry, marking it as dirty for the next SSBO update. If the entity is not registered, it will be added to the registry.
    pub fn update_entity(&mut self, entity: Entity, transform: GlobalTransform) {
        let transform_id = self.add_entity(entity, transform); // Make sure the entity is registered and get its transform ID
        self.dirty_transforms
            .push((transform_id, TransformUniform::new(&transform)));
    }

    /// Unregister an entity from the registry, freeing its transform ID for future use.
    pub fn remove_entity(&mut self, entity: Entity) {
        if let Some(transform_id) = self.entity_to_transform.remove(&entity) {
            self.available_transform_ids.push(transform_id);
        }
    }
}

// Update the list of dirty transforms for this frame
#[allow(clippy::type_complexity)]
fn set_dirty_transforms(
    mut registry: ResMut<PbrSsboTransformRegistry>,
    new_transforms: Query<
        (Entity, &GlobalTransform),
        Or<(
            (With<PbrSsboTransformMarker>, Added<GlobalTransform>),
            (With<GlobalTransform>, Added<PbrSsboTransformMarker>),
        )>,
    >,
    changed_transforms: Query<
        (Entity, &GlobalTransform),
        (With<PbrSsboTransformMarker>, Changed<GlobalTransform>),
    >,
    mut removed_transforms: RemovedComponents<PbrSsboTransformMarker>,
) {
    // Clear the list of dirty transforms from the previous frame
    registry.dirty_transforms.clear();

    // Add new or changed transforms to the dirty list
    for (entity, transform) in new_transforms.iter().chain(changed_transforms.iter()) {
        registry.update_entity(entity, *transform);
    }

    // Remove transforms that are no longer needed
    for entity in removed_transforms.read() {
        registry.remove_entity(entity);
    }
}

// Update the ssbo with the dirty transforms from the registry
fn update_ssbo_transforms(
    ssbo: ResRenderData<PbrSsboTransform>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    registry: Res<PbrSsboTransformRegistry>,
    render_instance: Res<RenderInstance>,
) {
    // Return early if there are no dirty transforms to update
    if registry.dirty_transforms.is_empty() {
        return;
    }

    // Get the ssbo cpu buffer and gpu buffer
    let (ssbo_staging, ssbo_gpu) = match (
        ssbo.iter().next().and_then(|(_, d)| buffers.get(&d.get_buffer(PbrSsboTransform::TRANSFORM_STAGING_IDX)?)),
        ssbo.iter().next().and_then(|(_, d)| buffers.get(&d.get_buffer(PbrSsboTransform::TRANSFORM_IDX)?))
    ) {
        (Some(staging), Some(gpu)) => (&staging.buffer, &gpu.buffer),
        _ => return,
    };

    // Write the dirty transforms to the staging buffer
    let render_instance = render_instance.0.read().unwrap();
    for (transform_id, transform) in &registry.dirty_transforms {
        let offset = (*transform_id as usize) * std::mem::size_of::<TransformUniform>();
        ssbo_staging.write(&render_instance, bytemuck::cast_slice(&[*transform]), offset);
    }

    // Copy the staging buffer to the GPU buffer
    ssbo_gpu.copy_from_buffer(&render_instance, ssbo_staging);
}
