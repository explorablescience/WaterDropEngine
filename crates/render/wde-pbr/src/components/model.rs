use std::collections::HashMap;

use bevy::prelude::*;
use wde_logger::prelude::*;
use wde_renderer::prelude::*;

use crate::{assets::PbrMaterial, logic::ssbo::MAX_ENTITY_COUNT};

/// A weak reference to a PbrModel, suitable for extraction to the render world
pub type PbrModelElementUuid = u128; // Unique identifier for each model element in the world 

/// A PBR model component that holds references to meshes and their associated PBR materials.
#[derive(Component, Debug, Clone)]
#[require(Transform)]
pub struct PbrModel(pub Vec<(Handle<Mesh>, Handle<PbrMaterial>)>);

#[derive(Resource, Default)]
pub struct ModelUuidToTransformUuidRender(pub HashMap<PbrModelElementUuid, u32>);

/// Resource to manage PBR render entities and their UUIDs.
/// This resource is used to keep track of the mapping between entities, their model element UUIDs, and the corresponding mesh and material handles. It also manages the mapping between model element UUIDs and their transform IDs in the SSBO.
#[derive(Resource, Default, Clone)]
pub struct PbrModelRegistry {
    /// Maps between entities and their model element UUIDs
    pub entity_to_model_uuids: HashMap<Entity, Vec<PbrModelElementUuid>>,
    /// Maps between each model element UUID and its mesh and material handles
    pub model_uuid_to_weak: HashMap<PbrModelElementUuid, (AssetId<Mesh>, AssetId<PbrMaterial>)>,

    /// Maps between model element UUIDs and their transform IDs in the SSBO
    pub model_uuid_to_transform_id: HashMap<PbrModelElementUuid, u32>,

    /// The next available UUID for a model element
    pub next_uuid: PbrModelElementUuid
}
impl PbrModelRegistry {
    fn next_uuid(&mut self) -> PbrModelElementUuid {
        let uuid = self.next_uuid;
        self.next_uuid += 1;
        uuid
    }

    /// Register a new entity with its mesh and material handles, and allocate a transform ID for it in the SSBO. Returns the generated UUID for the model element.
    fn register_entity(
        &mut self,
        entity: Entity,
        mesh_handle: &Handle<Mesh>,
        material_handle: &Handle<PbrMaterial>,
        transform_id: u32
    ) -> PbrModelElementUuid {
        let uuid = self.next_uuid();
        self.model_uuid_to_weak
            .insert(uuid, (mesh_handle.id(), material_handle.id()));
        self.entity_to_model_uuids
            .entry(entity)
            .or_default()
            .push(uuid);
        self.model_uuid_to_transform_id.insert(uuid, transform_id);
        uuid
    }
}

#[derive(Resource, Default)]
pub struct PbrSsboIdHandler {
    /// Index of the last used transform ID
    last_transform_id: u32,
    /// List of free transform IDs
    free_transform_ids: Vec<u32>
}
impl PbrSsboIdHandler {
    /// Allocate a new transform ID
    pub fn allocate_transform_id(&mut self) -> u32 {
        if let Some(free_id) = self.free_transform_ids.pop() {
            free_id
        } else if (self.last_transform_id as usize) < MAX_ENTITY_COUNT {
            let id = self.last_transform_id;
            self.last_transform_id += 1;
            id
        } else {
            warn!(
                "PbrSsbo: Maximum number of transform IDs reached ({})",
                MAX_ENTITY_COUNT
            );
            0
        }
    }

    /// Free a transform ID
    pub fn free_transform_id(&mut self, transform_id: u32) {
        self.free_transform_ids.push(transform_id);
    }
}

/// Resource to track dirty transforms that need to be updated in the SSBO.
/// This automatically updates every frame with the modified transforms of entities with a PbrModel component, and is used to update the SSBO `SsboTransformPbr` with the new transforms.
#[derive(Resource, Default)]
pub struct DirtyTransforms(pub Option<Vec<(PbrModelElementUuid, TransformUniform)>>);

pub struct PbrModelRegistryPlugin;
impl Plugin for PbrModelRegistryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PbrModelRegistry>()
            .init_resource::<PbrSsboIdHandler>()
            .init_resource::<DirtyTransforms>()
            .add_systems(Update, on_models_updates);
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<ModelUuidToTransformUuidRender>()
            .init_resource::<DirtyTransforms>();
    }
}

fn on_models_updates(
    modified_models_query: Query<(Entity, &Transform, &PbrModel), Changed<PbrModel>>,
    modified_transforms_query: Query<(Entity, &Transform), (Changed<Transform>, With<PbrModel>)>,
    mut removed: RemovedComponents<PbrModel>,
    mut registry: ResMut<PbrModelRegistry>,
    mut ssbo: ResMut<PbrSsboIdHandler>,
    mut dirty_transforms: ResMut<DirtyTransforms>
) {
    let mut new_dirty_transforms = Vec::new();

    // Handle modified PbrModel components
    for (entity, transform, pbr_model) in modified_models_query.iter() {
        // Remove existing uuids and free their transform IDs
        if let Some(uuids) = registry.entity_to_model_uuids.remove(&entity) {
            for uuid in uuids.iter() {
                if let Some(transform_id) = registry.model_uuid_to_transform_id.remove(uuid) {
                    ssbo.free_transform_id(transform_id);
                }
                registry.model_uuid_to_weak.remove(uuid);
            }
        }

        // Register the new model elements
        for (mesh_handle, material_handle) in pbr_model.0.iter() {
            let uuid = registry.register_entity(
                entity,
                mesh_handle,
                material_handle,
                ssbo.allocate_transform_id()
            );
            new_dirty_transforms.push((uuid, TransformUniform::new(transform)));
        }
    }

    // Handle modified Transforms
    for (entity, transform) in modified_transforms_query.iter() {
        if let Some(uuids) = registry.entity_to_model_uuids.get(&entity) {
            for uuid in uuids.iter() {
                new_dirty_transforms.push((*uuid, TransformUniform::new(transform)));
            }
        }
    }

    // Handle deleted PbrModel components
    for entity in removed.read() {
        // Remove the uuids associated with the entity
        if let Some(uuids) = registry.entity_to_model_uuids.remove(&entity) {
            // Free the transform IDs in the SSBO
            for uuid in uuids.iter() {
                if let Some(transform_id) = registry.model_uuid_to_transform_id.remove(uuid) {
                    ssbo.free_transform_id(transform_id);
                }
            }

            // Remove the weak references
            for uuid in uuids {
                registry.model_uuid_to_weak.remove(&uuid);
            }
        }
    }

    // Set the dirty transforms resource
    dirty_transforms.0 = Some(new_dirty_transforms.to_vec());
}
