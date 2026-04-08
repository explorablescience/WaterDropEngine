use wde_logger::prelude::*;

use bevy::prelude::*;
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::{
    deferred::{
        batches::*,
        transform::{PbrSsboTransform, ssbo_transforms::PbrSsboTransformStaging}
    },
    prelude::{
        model::*,
        ssbo_batches::{PbrSsboBatches, PbrSsboBatchesStaging}
    }
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
                    update_ssbo_transforms,
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
    mut model_uuid_to_transform_id: ResMut<ModelUuidToTransformUuidRender>,
    main_dirty_transforms: ExtractWorld<Res<DirtyTransforms>>,
    mut render_dirty_transforms: ResMut<DirtyTransforms>
) {
    // Extract the model UUIDs and their associated mesh and material weak references from the main world to the render world
    model_uuid_to_transform_id.0 = pbr_model_registry.model_uuid_to_transform_id.clone();
    render_dirty_transforms.0 = main_dirty_transforms.0.clone();

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

fn update_ssbo_transforms(
    buffers: Res<RenderAssets<GpuBuffer>>,
    ssbo: Binding<PbrSsboTransform>,
    ssbo_staging: Binding<PbrSsboTransformStaging>,
    render_instance: Res<RenderInstance>,
    mut dirty_transforms: ResMut<DirtyTransforms>,
    registry: Res<ModelUuidToTransformUuidRender>
) {
    let render_instance = render_instance.0.read().unwrap();

    // Take dirty transforms
    let dirty_transforms = match dirty_transforms.0.take() {
        Some(transforms) => transforms,
        None => return
    };
    if dirty_transforms.is_empty() {
        return;
    }

    // Get the ssbo cpu buffer
    let (ssbo, ssbo_staging) = match (ssbo.iter().next(), ssbo_staging.iter().next()) {
        (Some((_, ssbo)), Some((_, ssbo_staging))) => (ssbo, ssbo_staging),
        _ => return
    };
    let (ssbo_staging, ssbo_gpu) = match (
        buffers.get(ssbo_staging.get_buffer(0).unwrap()),
        buffers.get(ssbo.get_buffer(0).unwrap())
    ) {
        (Some(cpu), Some(gpu)) => (cpu, gpu),
        _ => return
    };

    {
        let _span = debug_span!("update_pbr_ssbo_buffer").entered();

        // Get the list of dirty transforms
        // Update the dirty transforms in the ssbo buffer
        for (uuid, transform) in dirty_transforms.iter() {
            // Get the transform ID
            let transform_id = match registry.0.get(uuid) {
                Some(tid) => *tid,
                None => {
                    warn!("No transform ID found for PbrModelElementUuid {}.", uuid);
                    0
                }
            };

            // Write the transform uniform directly to the staging buffer
            let offset = (transform_id as usize) * std::mem::size_of::<TransformUniform>();
            ssbo_staging.buffer.write(
                &render_instance,
                bytemuck::cast_slice(&[*transform]),
                offset
            );
        }
    }

    // Update the ssbo from the cpu buffer
    ssbo_gpu
        .buffer
        .copy_from_buffer(&render_instance, &ssbo_staging.buffer);
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
    ssbo: Binding<PbrSsboBatches>,
    ssbo_staging: Binding<PbrSsboBatchesStaging>,
    render_instance: Res<RenderInstance>,
    batches: Res<Batches>
) {
    let render_instance = render_instance.0.read().unwrap();

    // Get the batches instances buffers
    let (ssbo, ssbo_staging) = match (ssbo.iter().next(), ssbo_staging.iter().next()) {
        (Some((_, ssbo)), Some((_, ssbo_staging))) => (ssbo, ssbo_staging),
        _ => return
    };
    let (ssbo_instance_to_transform_buffer, instance_to_transform_gpu) = match (
        buffers.get(ssbo_staging.get_buffer(0).unwrap()),
        buffers.get(ssbo.get_buffer(0).unwrap())
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
        ssbo_instance_to_transform_buffer.buffer.write(
            &render_instance,
            bytemuck::cast_slice(&batches.transform_ids),
            0
        );
    }

    // Update the ssbo from the cpu buffer
    instance_to_transform_gpu
        .buffer
        .copy_from_buffer(&render_instance, &ssbo_instance_to_transform_buffer.buffer);
}
