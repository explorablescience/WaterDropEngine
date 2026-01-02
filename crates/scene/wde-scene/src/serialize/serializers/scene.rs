use crate::IGNORED_COMPONENTS;
use bevy::{log, prelude::*, reflect::serde::ReflectSerializer};

/// Serialize the scene of the world
/// 
/// # Returns
/// A string representing the serialized scene in JSON format
pub fn serialize_scene(world: &mut World) -> String {
    let entities = serialize_entities(world);
    serde_json::to_string(&entities).unwrap()
}


/// Serialize all entities in the world
/// 
/// # Returns
/// A vector of serialized entities as JSON strings
fn serialize_entities(world: &mut World) -> Vec<Vec<String>> {
    // Get the type registry (store of all registered types)
    let app_registry = world.resource::<AppTypeRegistry>().clone();
    let app_registry = app_registry.read();

    // Select all entities in the world
    let mut query = QueryBuilder::<EntityRef>::new(world);
    let mut query = query.build();

    // Iterate over all entities
    let mut entities = Vec::new();
    for entity in query.iter(world) {
        let archetype = entity.archetype();
        let mut entity_components = Vec::new();

        // Iterate over all components
        for component_id in archetype.iter_components() {
            // Find name and type id of the component
            let (name, type_id) = match world.components().get_info(component_id) {
                Some(info) => match info.type_id() {
                    Some(type_id) => (info.name(), type_id),
                    None => {
                        log::warn!("Component {} has no TypeId, skipping serialization.", info.name());
                        continue;
                    }
                },
                None => {
                    log::warn!("Could not get component info for id {:?}, skipping serialization.", component_id);
                    continue;
                }
            };

            // Skip ignored components
            if IGNORED_COMPONENTS.contains(&name.to_string().as_str()) {
                continue;
            }
            log::info!("Serializing component {} for entity {:?}", name, entity.id());

            // Get the component data as Reflect
            let component = match app_registry.get(type_id) {
                Some(type_registration) => {
                    match type_registration.data::<ReflectComponent>() {
                        Some(reflect_component) => match reflect_component.reflect(entity) {
                            Some(component) => component,
                            None => {
                                log::warn!("Could not reflect component {} for entity {:?}, skipping serialization.", name, entity.id());
                                continue;
                            }
                        },
                        None => {
                            log::warn!("Component {} does not implement ReflectComponent, skipping serialization.", name);
                            continue;
                        }
                    }
                },
                None => {
                    log::warn!("No type registration found for component {} with TypeId {:?}, skipping serialization.", name, type_id);
                    continue;
                }
            };
            
            // Serialize the component
            let serializer = ReflectSerializer::new(component, &app_registry);
            let serialized_component = serde_json::to_string(&serializer).unwrap();
            entity_components.push(serialized_component);
        }

        // Combine all components into a single JSON object
        if entity_components.is_empty() {
            continue;
        }
        entities.push(entity_components);
    }
    entities
}
