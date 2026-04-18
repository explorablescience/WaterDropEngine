use bevy::prelude::*;
use wde_egui::prelude::*;

use crate::{panels::ecs::entities_edit_components::SelectedEntity, ui::UIMenu};

const OBSERVER_COMPONENT_NAME: &str = "bevy_ecs::observer::distributed_storage::Observer";

#[derive(Clone)]
struct EntityRow {
    entity: Entity,
    name: Option<String>,
    parent: Option<Entity>
}

pub fn draw_entities_panel(world: &mut World) {
    let ctx = world.resource::<EguiContext>().0.clone();
    let mut selected = SelectedEntity(world.resource::<SelectedEntity>().0);
    let mut entities_panel_open = world.resource::<UIMenu>().is_clicked("Engine/Entities");

    // Collect entities once, then build a parent->children map used by the tree renderer.
    let rows = collect_entity_rows(world);
    let labels_by_entity = build_labels_map(&rows);
    let (roots, children_by_parent) = build_tree_index(&rows);

    // Draw the entities panel.
    egui::Window::new("Entities")
        .default_size([500.0, 440.0])
        .open(&mut entities_panel_open)
        .show(&ctx, |ui| {
            ui.label(format!("Total active entities: {}", rows.len()));
            if let Some(entity) = selected.0 {
                let selected_label = labels_by_entity
                    .get(&entity)
                    .cloned()
                    .unwrap_or_else(|| format!("{:?}", entity));
                ui.label(format!("Selected: {}", selected_label));
            }
            ui.separator();

            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    let mut visited = std::collections::HashSet::<Entity>::new();
                    for root in &roots {
                        render_entity_tree_node(
                            ui,
                            *root,
                            &labels_by_entity,
                            &children_by_parent,
                            &mut selected,
                            &mut visited
                        );
                    }

                    for row in &rows {
                        if !visited.contains(&row.entity) {
                            render_entity_tree_node(
                                ui,
                                row.entity,
                                &labels_by_entity,
                                &children_by_parent,
                                &mut selected,
                                &mut visited
                            );
                        }
                    }
                });
        });

    world.resource_mut::<SelectedEntity>().0 = selected.0;
    *world
        .resource_mut::<UIMenu>()
        .clicked_mut("Engine/Entities") = entities_panel_open;
}

fn collect_entity_rows(world: &mut World) -> Vec<EntityRow> {
    // Keep this query local so World borrows stay short and obvious.
    let mut entities_query = world.query::<(
        Entity,
        bevy::ecs::world::EntityRef,
        Option<&Name>,
        Option<&ChildOf>
    )>();
    let components = world.components();

    let mut rows = entities_query
        .iter(world)
        .filter_map(|(entity, entity_ref, name, parent)| {
            if is_observer_only_entity(entity_ref, components) {
                return None;
            }

            Some(EntityRow {
                entity,
                name: name.map(ToString::to_string),
                parent: parent.map(|parent| parent.parent())
            })
        })
        .collect::<Vec<_>>();

    rows.sort_by_key(|row| row.entity.index());
    rows
}

fn is_observer_only_entity(
    entity_ref: bevy::ecs::world::EntityRef,
    components: &bevy::ecs::component::Components
) -> bool {
    let mut component_names = entity_ref
        .archetype()
        .components()
        .iter()
        .filter_map(|component_id| components.get_name(*component_id));

    matches!(
        (component_names.next(), component_names.next()),
        (Some(component_name), None)
            if component_name.to_string() == OBSERVER_COMPONENT_NAME
    )
}

fn build_labels_map(rows: &[EntityRow]) -> std::collections::HashMap<Entity, String> {
    rows.iter()
        .map(|row| {
            (
                row.entity,
                row.name
                    .clone()
                    .unwrap_or_else(|| format!("{:?}", row.entity))
            )
        })
        .collect()
}

fn build_tree_index(
    rows: &[EntityRow]
) -> (Vec<Entity>, std::collections::HashMap<Entity, Vec<Entity>>) {
    // HashSet avoids repeated O(n) checks when validating parents.
    let entity_set = rows
        .iter()
        .map(|row| row.entity)
        .collect::<std::collections::HashSet<_>>();

    let mut roots = Vec::new();
    let mut children_by_parent = std::collections::HashMap::<Entity, Vec<Entity>>::new();
    for row in rows {
        match row.parent {
            Some(parent) if entity_set.contains(&parent) => {
                children_by_parent
                    .entry(parent)
                    .or_default()
                    .push(row.entity);
            }
            _ => roots.push(row.entity)
        }
    }

    roots.sort_by_key(|entity| entity.index());
    for children in children_by_parent.values_mut() {
        children.sort_by_key(|entity| entity.index());
    }

    (roots, children_by_parent)
}

fn render_entity_tree_node(
    ui: &mut egui::Ui,
    entity: Entity,
    labels_by_entity: &std::collections::HashMap<Entity, String>,
    children_by_parent: &std::collections::HashMap<Entity, Vec<Entity>>,
    selected: &mut SelectedEntity,
    visited: &mut std::collections::HashSet<Entity>
) {
    if !visited.insert(entity) {
        return;
    }

    let label = labels_by_entity
        .get(&entity)
        .cloned()
        .unwrap_or_else(|| format!("{:?}", entity));

    if let Some(children) = children_by_parent.get(&entity)
        && !children.is_empty()
    {
        let open_id = ui.make_persistent_id(("entity_tree_open", entity));
        let mut is_open = ui.memory_mut(|memory| {
            memory
                .data
                .get_persisted::<bool>(open_id)
                .unwrap_or_default()
        });

        ui.horizontal(|ui| {
            let toggle_icon = if is_open { "▼" } else { "▶" };
            if ui
                .add(egui::Button::new(toggle_icon).frame(false))
                .clicked()
            {
                is_open = !is_open;
            }

            if ui
                .selectable_label(selected.0 == Some(entity), label)
                .clicked()
            {
                selected.0 = Some(entity);
            }
        });

        ui.memory_mut(|memory| {
            memory.data.insert_persisted(open_id, is_open);
        });

        if is_open {
            ui.indent(open_id.with("children"), |ui| {
                for child in children {
                    render_entity_tree_node(
                        ui,
                        *child,
                        labels_by_entity,
                        children_by_parent,
                        selected,
                        visited
                    );
                }
            });
        } else {
            for child in children {
                mark_entity_subtree_visited(*child, children_by_parent, visited);
            }
        }
    } else if ui
        .selectable_label(selected.0 == Some(entity), label)
        .clicked()
    {
        selected.0 = Some(entity);
    }
}

fn mark_entity_subtree_visited(
    entity: Entity,
    children_by_parent: &std::collections::HashMap<Entity, Vec<Entity>>,
    visited: &mut std::collections::HashSet<Entity>
) {
    if !visited.insert(entity) {
        return;
    }

    if let Some(children) = children_by_parent.get(&entity) {
        for child in children {
            mark_entity_subtree_visited(*child, children_by_parent, visited);
        }
    }
}
