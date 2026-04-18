use bevy::prelude::*;
use wde_egui::prelude::*;

use crate::ui::UIMenu;

#[derive(Resource, Default)]
pub struct SelectedEntity(pub Option<Entity>);

#[derive(Resource, Default)]
pub struct SelectedComponent(pub Option<bevy::ecs::component::ComponentId>);

struct ComponentRow {
    id: bevy::ecs::component::ComponentId,
    name: String
}

pub fn draw_selected_entity_components_panel(world: &mut World) {
    let show_panel = world.resource::<UIMenu>().is_clicked("Engine/Entities");
    if !show_panel {
        return;
    }

    let selected = world.resource::<SelectedEntity>().0;
    let mut selected_component = world
        .get_resource::<SelectedComponent>()
        .and_then(|selected_component| selected_component.0);

    let mut component_rows = Vec::<ComponentRow>::new();
    let mut selected_label = None::<String>;
    let mut selected_alive = false;
    let mut selected_component_reflect_dump = None::<String>;
    let mut selected_component_message = None::<String>;

    if let Some(entity) = selected
        && world.entities().contains(entity)
    {
        selected_alive = true;
        selected_label = world
            .get::<Name>(entity)
            .map(ToString::to_string)
            .or_else(|| Some(format!("{:?}", entity)));

        let entity_ref = world.entity(entity);
        let components = world.components();

        component_rows = entity_ref
            .archetype()
            .components()
            .iter()
            .filter_map(|component_id| {
                components.get_name(*component_id).map(|name| ComponentRow {
                    id: *component_id,
                    name: name.to_string()
                })
            })
            .collect();
        component_rows.sort_by(|left, right| left.name.cmp(&right.name));

        if selected_component
            .is_some_and(|selected_id| !component_rows.iter().any(|row| row.id == selected_id))
        {
            selected_component = None;
        }

        if let Some(component_id) = selected_component {
            match component_reflect_dump(world, entity, component_id) {
                ComponentReflectDump::Dump(dump) => {
                    selected_component_reflect_dump = Some(dump);
                }
                ComponentReflectDump::Message(message) => {
                    selected_component_message = Some(message);
                }
            }
        }
    }

    if selected.is_some() && !selected_alive {
        world.resource_mut::<SelectedEntity>().0 = None;
        selected_component = None;
    }

    let ctx = world.resource::<EguiContext>();
    egui::Window::new("ECS - Selected Entity Components")
        .default_size([420.0, 440.0])
        .show(&ctx.0, |ui| {
            if let Some(entity) = selected {
                if selected_alive {
                    ui.label(format!(
                        "Entity: {}",
                        selected_label.unwrap_or_else(|| format!("{:?}", entity))
                    ));
                    ui.label(format!("Components: {}", component_rows.len()));
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .id_salt("ecs_components_list_scroll")
                        .show(ui, |ui| {
                            for component in &component_rows {
                                if ui
                                    .selectable_label(
                                        selected_component == Some(component.id),
                                        &component.name
                                    )
                                    .clicked()
                                {
                                    selected_component = Some(component.id);
                                }
                            }
                        });

                    ui.separator();
                    if let Some(component_id) = selected_component {
                        let selected_component_name = component_rows
                            .iter()
                            .find(|row| row.id == component_id)
                            .map(|row| row.name.as_str())
                            .unwrap_or("Unknown Component");

                        ui.label(format!("Selected component: {selected_component_name}"));

                        if let Some(reflect_dump) = selected_component_reflect_dump.as_deref() {
                            egui::ScrollArea::vertical()
                                .id_salt(("ecs_component_reflect_scroll", entity, component_id))
                                .max_height(220.0)
                                .show(ui, |ui| {
                                    ui.code(reflect_dump);
                                });
                        } else if let Some(message) = selected_component_message.as_deref() {
                            ui.label(message);
                        } else {
                            ui.label("Select a component to inspect reflected properties.");
                        }
                    }
                } else {
                    ui.label("Selected entity no longer exists.");
                }
            } else {
                ui.label("Select an entity in the Active Entities panel.");
            }
        });

    world.insert_resource(SelectedComponent(selected_component));
}

enum ComponentReflectDump {
    Dump(String),
    Message(String)
}

fn component_reflect_dump(
    world: &World,
    entity: Entity,
    component_id: bevy::ecs::component::ComponentId
) -> ComponentReflectDump {
    let Some(component_info) = world.components().get_info(component_id) else {
        return ComponentReflectDump::Message("Component metadata is unavailable.".to_string());
    };

    let Some(type_id) = component_info.type_id() else {
        return ComponentReflectDump::Message(
            "Component has no Rust TypeId available for reflection.".to_string()
        );
    };

    let type_registry = world.resource::<AppTypeRegistry>().read();
    let Some(type_registration) = type_registry.get(type_id) else {
        return ComponentReflectDump::Message(
            "Type is not registered in AppTypeRegistry.".to_string()
        );
    };

    let Some(reflect_component) = type_registration.data::<bevy::ecs::reflect::ReflectComponent>()
    else {
        return ComponentReflectDump::Message(
            "Component does not provide ReflectComponent metadata.".to_string()
        );
    };

    let entity_ref = world.entity(entity);
    let Some(reflected_component) = reflect_component.reflect(entity_ref) else {
        return ComponentReflectDump::Message(
            "Unable to access reflected data for this entity component.".to_string()
        );
    };

    ComponentReflectDump::Dump(format!("{:#?}", reflected_component))
}
