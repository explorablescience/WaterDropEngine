use std::collections::HashMap;
use uuid::Uuid;

use bevy::prelude::*;
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::prelude::PbrSsboTransform;

pub(crate) struct SsboTransformRegistryPlugin;
impl Plugin for SsboTransformRegistryPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<PbrSsboTransformUuid>::default(),
            ExtractResourcePlugin::<PbrSsboTransformPendingRemovals>::default()
        ))
        .init_resource::<PbrSsboTransformPendingRemovals>()
        .init_resource::<MainWorldEntityUuidMap>()
        .add_systems(
            Update,
            (MainWorldEntityUuidMap::add, MainWorldEntityUuidMap::remove).chain()
        );
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<PbrUuidRegistry>()
            .add_systems(Render, update_registry.in_set(RenderSet::Prepare));
    }
}

/// Marker component to track the UUID of an entity's transform in the SSBO registry
#[derive(Component, Clone)]
pub struct PbrSsboTransformUuid(pub Uuid);
impl Default for PbrSsboTransformUuid {
    fn default() -> Self {
        PbrSsboTransformUuid(Uuid::new_v4())
    }
}
impl SyncComponent for PbrSsboTransformUuid {
    type Target = Self;
}
impl ExtractComponent for PbrSsboTransformUuid {
    type QueryData = (&'static GlobalTransform, &'static Self);
    type QueryFilter = Changed<GlobalTransform>;
    type Out = (GlobalTransform, Self);

    fn extract_component(
        (transform, uuid): QueryItem<'_, '_, Self::QueryData>
    ) -> Option<Self::Out> {
        Some((*transform, uuid.clone()))
    }
}

/// Similar to `PbrSsboTransformUuid`, but tracks the entity that were removed last frame
#[derive(Resource, Default, Clone, ExtractResource)]
struct PbrSsboTransformPendingRemovals(pub Vec<Uuid>);
#[derive(Resource, Default)]
struct MainWorldEntityUuidMap(HashMap<Entity, Uuid>);
impl MainWorldEntityUuidMap {
    pub fn add(
        mut map: ResMut<Self>,
        added: Query<(Entity, &PbrSsboTransformUuid), Added<PbrSsboTransformUuid>>
    ) {
        for (entity, uuid) in added.iter() {
            map.0.insert(entity, uuid.0);
        }
    }

    pub fn remove(
        mut map: ResMut<Self>,
        mut pending_removals: ResMut<PbrSsboTransformPendingRemovals>,
        mut removed: RemovedComponents<PbrSsboTransformUuid>
    ) {
        pending_removals.0.clear();
        for entity in removed.read() {
            if let Some(uuid) = map.0.remove(&entity) {
                pending_removals.0.push(uuid);
            }
        }
    }
}

/// Resource to track the mapping between render entities and their transform IDs in the SSBO.
#[derive(Resource, Default, Clone)]
pub struct PbrUuidRegistry {
    pub uuid_to_transform: HashMap<Uuid, u32>, // (uuid, transform_id in the SSBO)
    pub available_transform_ids: Vec<u32>,     // List of available transform IDs in the SSBO
    pub next_transform_id: u32                 // Next available transform ID in the SSBO
}
impl PbrUuidRegistry {
    /// Get the transform ID for a given UUID, adding it to the registry if it doesn't exist.
    pub fn get_or_add(&mut self, uuid: Uuid) -> u32 {
        if let Some(&transform_id) = self.uuid_to_transform.get(&uuid) {
            transform_id
        } else {
            let transform_id = self.available_transform_ids.pop().unwrap_or_else(|| {
                let id = self.next_transform_id;
                self.next_transform_id += 1;
                id
            });
            self.uuid_to_transform.insert(uuid, transform_id);
            transform_id
        }
    }

    /// Remove a UUID from the registry, freeing its transform ID for future use.
    pub fn remove(&mut self, uuid: Uuid) {
        if let Some(transform_id) = self.uuid_to_transform.remove(&uuid) {
            self.available_transform_ids.push(transform_id);
        }
    }

    /// Get the transform ID for a given UUID, if it exists.
    pub fn get(&self, uuid: &Uuid) -> Option<u32> {
        self.uuid_to_transform.get(uuid).copied()
    }
}

#[derive(Component, Default)]
struct ChangedTransformToRetryMarker;
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_registry(
    mut commands: Commands,
    mut registry: ResMut<PbrUuidRegistry>,
    pending_removals: Res<PbrSsboTransformPendingRemovals>,
    changed_transforms: Query<
        (Entity, &PbrSsboTransformUuid, &GlobalTransform),
        (With<PbrSsboTransformUuid>, Changed<GlobalTransform>)
    >,
    changeds_to_retry: Query<
        (Entity, &PbrSsboTransformUuid, &GlobalTransform),
        (
            With<PbrSsboTransformUuid>,
            With<ChangedTransformToRetryMarker>
        )
    >,
    ssbo: ResRenderData<PbrSsboTransform>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    render_instance: Res<RenderInstance>
) {
    // Free transform IDs for removed entities
    for uuid in &pending_removals.0 {
        registry.remove(*uuid);
    }

    // Return early if there are no changed transforms to update
    if changed_transforms.is_empty() && changeds_to_retry.is_empty() {
        return;
    }

    // Update transform IDs for changed entities
    let (ssbo_staging, ssbo_gpu) = match (
        ssbo.iter().next().and_then(|(_, d)| {
            buffers.get(&d.get_buffer(PbrSsboTransform::TRANSFORM_STAGING_IDX)?)
        }),
        ssbo.iter()
            .next()
            .and_then(|(_, d)| buffers.get(&d.get_buffer(PbrSsboTransform::TRANSFORM_IDX)?))
    ) {
        (Some(staging), Some(gpu)) => (&staging.buffer, &gpu.buffer),
        _ => {
            for (entity, _, _) in changed_transforms.iter().chain(changeds_to_retry.iter()) {
                commands
                    .entity(entity)
                    .insert(ChangedTransformToRetryMarker);
            }
            return;
        }
    };
    let render_instance = render_instance.0.read().unwrap();
    for (entity, uuid, transform) in changed_transforms.iter().chain(changeds_to_retry.iter()) {
        let transform_id = registry.get_or_add(uuid.0);
        let offset = (transform_id as usize) * std::mem::size_of::<TransformUniform>();
        ssbo_staging.write(
            &render_instance,
            bytemuck::cast_slice(&[TransformUniform::new(transform)]),
            offset
        );
        commands
            .entity(entity)
            .remove::<ChangedTransformToRetryMarker>();
    }
    ssbo_gpu.copy_from_buffer(&render_instance, ssbo_staging);
}
