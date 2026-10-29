use std::{collections::HashMap, ops::Range};

use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::prelude::{
    CULL_VIEW_COUNT, PbrAlphaMode, PbrMaterial, PbrMaterial3d, PbrSsboTransformUuid,
    PbrUuidRegistry
};

#[derive(Component)]
pub struct ExtractedPbrInstance {
    pub(crate) mesh_id: AssetId<Mesh>,
    material_id: AssetId<PbrMaterial>
}

#[derive(Component)]
pub(crate) struct ExtractedPbrInstanceToRetryMarker;

/// Marker component to indicate that an entity should be included in the PBR render batches.
/// This will automatically add the [PbrSsboTransformMarker] to the entity, so that its transform will be included in the SSBO updates for rendering.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
#[require(PbrSsboTransformUuid)]
pub struct PbrBatchesMarker;
impl SyncComponent for PbrBatchesMarker {
    type Target = Self;
}
impl ExtractComponent for PbrBatchesMarker {
    type QueryData = (&'static Mesh3d, &'static PbrMaterial3d<PbrMaterial>);
    type QueryFilter = (
        With<PbrBatchesMarker>,
        With<Transform>,
        Or<(Changed<Mesh3d>, Changed<PbrMaterial3d<PbrMaterial>>)>
    );
    type Out = ExtractedPbrInstance;

    fn extract_component(
        (mesh, material): QueryItem<'_, '_, Self::QueryData>
    ) -> Option<Self::Out> {
        Some(ExtractedPbrInstance {
            mesh_id: mesh.0.id(),
            material_id: material.0.id()
        })
    }
}

// Key for batching, based on mesh and material. Entities with the same mesh and material will be batched together.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub(crate) struct BatchKey {
    pub(crate) mesh: AssetId<Mesh>,
    pub(crate) material: AssetId<PbrMaterial>
}
// Batch of entities that share the same mesh and material.
pub(crate) struct Batch {
    pub _key: BatchKey,
    /// Render entity and SSBO transform ID of each instance.
    pub instances: Vec<(Entity, u32)>,
    /// Instances visible in each culling view, as a range of the "instance to transform" buffer.
    pub views: [Range<u32>; CULL_VIEW_COUNT]
}
// Resource to store the batches of entities for rendering. The key is the combination of mesh and material, and the value is the batch of entities that share that mesh and material.
#[derive(Resource, Default)]
pub(crate) struct BatchList {
    pub batches: HashMap<BatchKey, Batch>,
    pub sorted_batches: Vec<BatchKey>, // List of batch keys sorted first by material and then by mesh
    pub(crate) tracked_entities: HashMap<Entity, BatchKey>
}
impl std::fmt::Debug for BatchList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct("BatchList");
        for batch_key in &self.sorted_batches {
            if let Some(batch) = self.batches.get(batch_key) {
                debug_struct.field(
                    &format!(
                        "Batch(mesh: {:?}, material: {:?})",
                        batch_key.mesh, batch_key.material
                    ),
                    &format!(
                        "Batch {{ instances: {:?}, views: {:?} }}",
                        batch.instances, batch.views
                    )
                );
            }
        }
        debug_struct.finish()
    }
}

/// Whether the batch material is alpha-blended, i.e. drawn by the transparent pass only.
pub(crate) fn is_batch_blended(
    materials: &RenderAssets<GpuRenderBinding<PbrMaterial>>,
    key: &BatchKey
) -> bool {
    materials
        .get(key.material)
        .is_some_and(|m| m.asset.alpha_mode == PbrAlphaMode::Blend)
}

fn remove_tracked_entity(batches: &mut BatchList, entity: Entity) -> bool {
    let Some(key) = batches.tracked_entities.remove(&entity) else {
        return false;
    };

    let mut removed_any = false;
    let mut remove_batch_key = false;
    if let Some(batch) = batches.batches.get_mut(&key) {
        let len_before = batch.instances.len();
        batch.instances.retain(|(e, _)| *e != entity);
        removed_any = batch.instances.len() != len_before;
        remove_batch_key = batch.instances.is_empty();
    }

    if remove_batch_key {
        batches.batches.remove(&key);
        batches.sorted_batches.retain(|k| *k != key);
    }

    removed_any
}

pub(crate) fn build_batches(
    mut commands: Commands,
    mut batches: ResMut<BatchList>,
    mut removed_extracted_instances: RemovedComponents<ExtractedPbrInstance>,
    mut removed_transform_uuid: RemovedComponents<PbrSsboTransformUuid>,
    extracted_instances: Query<
        (Entity, &PbrSsboTransformUuid, &ExtractedPbrInstance),
        Changed<ExtractedPbrInstance>
    >,
    extracted_instances_to_retry: Query<
        (Entity, &PbrSsboTransformUuid, &ExtractedPbrInstance),
        With<ExtractedPbrInstanceToRetryMarker>
    >,
    transform_registry: Res<PbrUuidRegistry>
) {
    let mut changed = false;

    for entity in removed_extracted_instances.read() {
        changed |= remove_tracked_entity(&mut batches, entity);
    }
    for entity in removed_transform_uuid.read() {
        changed |= remove_tracked_entity(&mut batches, entity);
    }

    // Add new or changed instances to the batches, and remove the retry marker if it exists
    for (entity, transform_uuid, instance) in extracted_instances
        .into_iter()
        .chain(extracted_instances_to_retry)
    {
        changed |= remove_tracked_entity(&mut batches, entity);

        let transform_id = if let Some(id) = transform_registry.get(&transform_uuid.0) {
            id
        } else {
            // Retry next frame
            commands
                .entity(entity)
                .insert(ExtractedPbrInstanceToRetryMarker);
            continue;
        };

        let key = BatchKey {
            mesh: instance.mesh_id,
            material: instance.material_id
        };
        batches
            .batches
            .entry(key)
            .or_insert_with(|| Batch {
                _key: key,
                instances: Vec::new(),
                views: Default::default()
            })
            .instances
            .push((entity, transform_id));
        batches.tracked_entities.insert(entity, key);
        if !batches.sorted_batches.contains(&key) {
            batches.sorted_batches.push(key);
        }
        commands
            .entity(entity)
            .remove::<ExtractedPbrInstanceToRetryMarker>();
        changed = true;
    }

    // Sort batches first by material and then by mesh
    if changed {
        batches.sorted_batches.sort_by(|a, b| {
            a.material
                .cmp(&b.material)
                .then_with(|| a.mesh.cmp(&b.mesh))
        });
    }
}

pub(crate) struct BatchesPlugin;
impl Plugin for BatchesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<PbrBatchesMarker>::default());
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<BatchList>();
    }
}
