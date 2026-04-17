use std::collections::HashMap;

use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::prelude::{PbrMaterial, PbrMaterial3d, PbrSsboTransformUuid, PbrUuidRegistry};

#[derive(Component)]
pub struct ExtractedPbrInstance {
    mesh_id: AssetId<Mesh>,
    material_id: AssetId<PbrMaterial>
}

#[derive(Component)]
pub(crate) struct ExtractedPbrInstanceToRetryMarker;

/// Marker component to indicate that an entity should be included in the PBR render batches.
/// This will automatically add the [PbrSsboTransformMarker] to the entity, so that its transform will be included in the SSBO updates for rendering.
#[derive(Component, Default)]
#[require(PbrSsboTransformUuid)]
pub struct PbrBatchesMarker;
impl SyncComponent for PbrBatchesMarker {
    type Target = Self;
}
impl ExtractComponent for PbrBatchesMarker {
    type QueryData = (&'static Mesh3d, &'static PbrMaterial3d);
    type QueryFilter = (With<PbrBatchesMarker>, With<Transform>, Or<(Changed<Mesh3d>, Changed<PbrMaterial3d>)>);
    type Out = ExtractedPbrInstance;

    fn extract_component((mesh, material): QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
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
// Batch of entities that share the same mesh and material, represented by their transform IDs in the SSBO.
pub(crate) struct Batch {
    pub _key: BatchKey,
    pub transform_ssbo_ids: Vec<u32>, // Offset in the SSBO "transform" buffer of the first transform of this batch
    pub instances_offset: u32,        // Offset in the SSBO "instance to transform" buffer of the first entity of this batch
}
// Resource to store the batches of entities for rendering. The key is the combination of mesh and material, and the value is the batch of transform IDs for entities that share that mesh and material.
#[derive(Resource, Default)]
pub(crate) struct BatchList {
    pub batches: HashMap<BatchKey, Batch>,
    pub sorted_batches: Vec<BatchKey>, // List of batch keys sorted first by material and then by mesh
    pub dirty_batches: Vec<BatchKey>   // List of batch keys that have been modified and need to be updated in the GPU instance to transform buffer
}
impl std::fmt::Debug for BatchList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct("BatchList");
        for batch_key in &self.sorted_batches {
            if let Some(batch) = self.batches.get(batch_key) {
                debug_struct.field(
                    &format!("Batch(mesh: {:?}, material: {:?})", batch_key.mesh, batch_key.material),
                    &format!("Batch {{ transform_ssbo_ids: {:?}, instances_offset: {} }}", batch.transform_ssbo_ids, batch.instances_offset)
                );
            }
        }
        debug_struct.finish()
    }
}

pub(crate) fn build_batches(
    mut commands: Commands,
    mut batches: ResMut<BatchList>,
    extracted_instances: Query<(Entity, &PbrSsboTransformUuid, &ExtractedPbrInstance), Changed<ExtractedPbrInstance>>,
    extracted_instances_to_retry: Query<(Entity, &PbrSsboTransformUuid, &ExtractedPbrInstance), With<ExtractedPbrInstanceToRetryMarker>>,
    transform_registry: Res<PbrUuidRegistry>
) {
    // Add new or changed instances to the batches, and remove the retry marker if it exists
    for (entity, transform_uuid, instance) in extracted_instances.into_iter().chain(extracted_instances_to_retry) {
        let transform_id = if let Some(id) = transform_registry.get(&transform_uuid.0) {
            id
        } else {
            // Retry next frame
            commands.entity(entity).insert(ExtractedPbrInstanceToRetryMarker);
            continue;
        };

        let key = BatchKey {
            mesh: instance.mesh_id,
            material: instance.material_id
        };
        batches.batches.entry(key).or_insert_with(|| Batch {
            _key: key,
            transform_ssbo_ids: Vec::new(),
            instances_offset: 0
        }).transform_ssbo_ids.push(transform_id);
        batches.sorted_batches.push(key);
        batches.dirty_batches.push(key);
        commands.entity(entity).remove::<ExtractedPbrInstanceToRetryMarker>();
    }

    // Sort batches first by material and then by mesh
    if extracted_instances.iter().next().is_some() {
        batches.sorted_batches.sort_by(|a, b| {
            a.material.cmp(&b.material).then_with(|| a.mesh.cmp(&b.mesh))
        });
    }
}

pub(crate) struct BatchesPlugin;
impl Plugin for BatchesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<PbrBatchesMarker>::default());
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<BatchList>();
    }
}
