use wde_logger::prelude::*;

use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::{
    prelude::{PbrMaterial, model::*, ssbo_batches::PbrSsboInstanceToTransform}
};

type ModelMaterialPair = (AssetId<Mesh>, AssetId<PbrMaterial>);

/// List of extracted render entities (UUIDs, mesh and material weak references, transform ID in SSBO)
#[derive(Resource, Default)]
struct ExtractedEntities(Option<Vec<(PbrModelElementUuid, ModelMaterialPair, u32)>>);

/// Store the render batches for PBR models
#[derive(Resource, Default, Debug)]
pub(crate) struct Batches {
    /// The render batches
    pub render_batches: Vec<Batch>,
    /// Pointers to the transform IDs in the SSBO for each instance in the batches
    pub transform_ids: Vec<u32>
}

/// A single render batch
#[derive(Debug, Default)]
pub(crate) struct Batch {
    pub mesh_id: AssetId<Mesh>,
    pub material_id: AssetId<PbrMaterial>,
    pub first_instance: u32,
    pub instance_count: u32
}

pub(crate) struct BatchesPlugin;
impl Plugin for BatchesPlugin {
    fn build(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<Batches>()
            .init_resource::<ExtractedEntities>()
            .add_systems(Extract, extract)
            .add_systems(
                Render,
                (
                    build_batches,
                    set_batches_transforms
                )
                    .in_set(RenderSet::Prepare)
            );
    }
}

// Extract the render entities from the main world to the render world
// As this function is sync, it should be fast
fn extract(
    raw_entities: ExtractWorld<Query<Entity, With<PbrModel>>>,
    model_registry: ExtractWorld<Res<PbrModelRegistry>>,
    mut extracted_entities: ResMut<ExtractedEntities>,
    pbr_model_registry: ExtractWorld<Res<PbrModelRegistry>>,
    mut model_uuid_to_transform_id: ResMut<ModelUuidToTransformUuidRender>
) {
    // Extract the model UUIDs and their associated mesh and material weak references from the main world to the render world
    model_uuid_to_transform_id.0 = pbr_model_registry.model_uuid_to_transform_id.clone();

    // Extract every uuid from the entities
    let mut render_entities = Vec::new();
    for entity in raw_entities.iter() {
        if let Some(uuid_list) = model_registry.entity_to_model_uuids.get(&entity) {
            for uuid in uuid_list.iter() {
                if let (Some(model_weak), Some(transform_id)) = (
                    model_registry.model_uuid_to_weak.get(uuid),
                    model_registry.model_uuid_to_transform_id.get(uuid)
                ) {
                    render_entities.push((*uuid, *model_weak, *transform_id));
                } else {
                    warn!(
                        "PbrModelRegistry has no weak reference or transform ID for model UUID {}",
                        uuid
                    );
                }
            }
        } else {
            warn!("PbrModelRegistry has no entry for entity {:?}", entity);
        }
    }

    // Store the extracted entities in a resource
    extracted_entities.0 = Some(render_entities);
}

fn build_batches(mut extracted_entities: ResMut<ExtractedEntities>, mut batches: ResMut<Batches>) {
    // Sort the entities first by mesh and then by material
    let _sort_span = debug_span!("build_batches_sort").entered();
    let mut entities = match extracted_entities.0.take() {
        Some(entities) => entities,
        None => return
    };
    entities.sort_by_key(|(_, model_weak, _)| (model_weak.0, model_weak.1));
    drop(_sort_span);

    // Create the batches
    let _batch_span = debug_span!("build_batches_create").entered();
    let mut render_batches: Vec<Batch> = Vec::new();
    let mut transform_ids: Vec<u32> = Vec::new();
    let mut current_mesh_id: Option<AssetId<Mesh>> = None;
    let mut current_material_id: Option<AssetId<PbrMaterial>> = None;
    for (_, (mesh_id, material_id), transform_id) in entities.iter() {
        // Add it to the corresponding batch
        if current_mesh_id == Some(*mesh_id) && current_material_id == Some(*material_id) {
            // Increment the instance count of the last batch
            if let Some(last_batch) = render_batches.last_mut() {
                last_batch.instance_count += 1;
            }
        } else {
            // Create a new batch
            render_batches.push(Batch {
                mesh_id: *mesh_id,
                material_id: *material_id,
                first_instance: render_batches
                    .last()
                    .map_or(0, |b| b.first_instance + b.instance_count),
                instance_count: 1
            });
            current_mesh_id = Some(*mesh_id);
            current_material_id = Some(*material_id);
        }

        // Add the transform ID (the index in the SSBO)
        transform_ids.push(*transform_id);
    }
    *batches = Batches {
        render_batches,
        transform_ids
    };
}

fn set_batches_transforms(
    buffers: Res<RenderAssets<GpuBuffer>>,
    ssbo: ResRenderData<PbrSsboInstanceToTransform>,
    render_instance: Res<RenderInstance>,
    batches: Res<Batches>
) {
    let render_instance = render_instance.0.read().unwrap();

    // Get the batches instances buffers
    let ssbo = match ssbo.iter().next() {
        Some((_, ssbo)) => ssbo,
        _ => return
    };
    let (ssbo_staging, ssbo_gpu) = match (
        buffers.get(
            &ssbo
                .get_buffer(PbrSsboInstanceToTransform::INSTANCE_TO_TRANSFORM_STAGING_IDX)
                .unwrap()
        ),
        buffers.get(
            &ssbo
                .get_buffer(PbrSsboInstanceToTransform::INSTANCE_TO_TRANSFORM_IDX)
                .unwrap()
        )
    ) {
        (Some(instance_to_transform_buffer), Some(instance_to_transform_gpu)) => {
            (instance_to_transform_buffer, instance_to_transform_gpu)
        }
        _ => return
    };

    // Fill the ssbo instance-to-transform
    {
        let _span = debug_span!("fill_instance_to_transform_ssbo").entered();

        // Write the instance-to-transform data directly
        ssbo_staging.buffer.write(
            &render_instance,
            bytemuck::cast_slice(&batches.transform_ids),
            0
        );
    }

    // Update the ssbo from the cpu buffer
    ssbo_gpu
        .buffer
        .copy_from_buffer(&render_instance, &ssbo_staging.buffer);
}
