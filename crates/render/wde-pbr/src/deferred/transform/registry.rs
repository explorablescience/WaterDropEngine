use bytemuck::Zeroable;
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
#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
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

/// Changed transforms closer than this (in IDs) are uploaded in the same write, unchanged ones
/// in between included: each `write_buffer` call costs a staging buffer allocation.
const MAX_UPLOAD_GAP: u32 = 64;

/// Resource to track the mapping between render entities and their transform IDs in the SSBO.
#[derive(Resource, Default, Clone)]
pub struct PbrUuidRegistry {
    pub uuid_to_transform: HashMap<Uuid, u32>, // (uuid, transform_id in the SSBO)
    pub available_transform_ids: Vec<u32>,     // List of available transform IDs in the SSBO
    pub next_transform_id: u32,                // Next available transform ID in the SSBO
    /// CPU copy of the SSBO transforms, to upload the changed ones in a few large writes.
    transforms: Vec<TransformUniform>
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

    let Some(ssbo_gpu) = ssbo
        .iter()
        .next()
        .and_then(|(_, d)| buffers.get(&d.get_buffer(PbrSsboTransform::TRANSFORM_IDX)?))
        .map(|gpu| &gpu.buffer)
    else {
        for (entity, _, _) in changed_transforms.iter() {
            commands
                .entity(entity)
                .insert(ChangedTransformToRetryMarker);
        }
        return;
    };
    let mut changed_ids = Vec::new();
    for (_, uuid, transform) in changed_transforms.iter().chain(changeds_to_retry.iter()) {
        let transform_id = registry.get_or_add(uuid.0);
        let index = transform_id as usize;
        if registry.transforms.len() <= index {
            registry
                .transforms
                .resize(index + 1, TransformUniform::zeroed());
        }
        registry.transforms[index] = TransformUniform::new(transform);
        changed_ids.push(transform_id);
    }
    changed_ids.sort_unstable();
    changed_ids.dedup();

    let render_instance = render_instance.0.read().unwrap();
    let mut ids = changed_ids.iter().copied().peekable();
    while let Some(first) = ids.next() {
        let mut last = first;
        while let Some(next) = ids.next_if(|&id| id - last <= MAX_UPLOAD_GAP) {
            last = next;
        }
        let range = first as usize..=last as usize;
        ssbo_gpu.write(
            &render_instance,
            bytemuck::cast_slice(&registry.transforms[range]),
            first as usize * std::mem::size_of::<TransformUniform>()
        );
    }
    for (entity, _, _) in changeds_to_retry.iter() {
        commands
            .entity(entity)
            .remove::<ChangedTransformToRetryMarker>();
    }
}
