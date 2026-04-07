use bevy::prelude::*;
use wde_egui::prelude::*;

use crate::ui::UIMenu;

#[derive(Resource, Default)]
pub struct SelectedEntity(Option<Entity>);

pub fn draw_entities_panel(
    ctx: Res<EguiContext>,
    mut ui_menu: ResMut<UIMenu>,
    entities: Query<(Entity, Option<&Name>, Option<&ChildOf>)>,
    mut selected: ResMut<SelectedEntity>
) {
    // Build a tree structure of entities based on their parent-child relationships
    let mut rows = entities
        .iter()
        .map(|(entity, name, parent)| {
            (
                entity,
                name.map(ToString::to_string),
                parent.map(|parent| parent.parent())
            )
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|(entity, _, _)| entity.index());

    // Identify root entities and build a mapping of parent to children
    let entity_set = rows
        .iter()
        .map(|(entity, _, _)| *entity)
        .collect::<Vec<_>>();
    let mut roots = Vec::new();
    let mut children_by_parent = std::collections::HashMap::<Entity, Vec<Entity>>::new();
    for (entity, _, parent) in &rows {
        match parent {
            Some(parent) if entity_set.contains(parent) => {
                children_by_parent.entry(*parent).or_default().push(*entity);
            }
            _ => roots.push(*entity)
        }
    }

    // Sort roots and children by entity index for consistent display order
    roots.sort_by_key(|entity| entity.index());
    for children in children_by_parent.values_mut() {
        children.sort_by_key(|entity| entity.index());
    }

    // Draw the UI panel with a tree view of entities
    egui::Window::new("Entities")
        .default_size([360.0, 440.0])
        .open(ui_menu.clicked_mut("Engine/Entities"))
        .show(&ctx.0, |ui| {
            ui.label(format!("Total active entities: {}", rows.len()));
            if let Some(entity) = selected.0 {
                let selected_label = rows
                    .iter()
                    .find(|(row_entity, _, _)| *row_entity == entity)
                    .map(|(_, name, _)| match name {
                        Some(name) => name.clone(),
                        None => format!("{:?}", entity)
                    })
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
                            0,
                            &rows,
                            &children_by_parent,
                            &mut selected,
                            &mut visited
                        );
                    }

                    for (entity, _, _) in &rows {
                        if !visited.contains(entity) {
                            render_entity_tree_node(
                                ui,
                                *entity,
                                0,
                                &rows,
                                &children_by_parent,
                                &mut selected,
                                &mut visited
                            );
                        }
                    }
                });
        });
}

fn render_entity_tree_node(
    ui: &mut egui::Ui,
    entity: Entity,
    depth: usize,
    rows: &[(Entity, Option<String>, Option<Entity>)],
    children_by_parent: &std::collections::HashMap<Entity, Vec<Entity>>,
    selected: &mut SelectedEntity,
    visited: &mut std::collections::HashSet<Entity>
) {
    if !visited.insert(entity) {
        return;
    }

    let label = rows
        .iter()
        .find(|(row_entity, _, _)| *row_entity == entity)
        .map(|(_, name, _)| match name {
            Some(name) => name.clone(),
            None => format!("{:?}", entity)
        })
        .unwrap_or_else(|| format!("{:?}", entity));

    ui.horizontal(|ui| {
        ui.add_space(depth as f32 * 16.0);
        if ui
            .selectable_label(selected.0 == Some(entity), label)
            .clicked()
        {
            selected.0 = Some(entity);
        }
    });

    if let Some(children) = children_by_parent.get(&entity) {
        for child in children {
            render_entity_tree_node(
                ui,
                *child,
                depth + 1,
                rows,
                children_by_parent,
                selected,
                visited
            );
        }
    }
}

pub fn draw_selected_entity_components_panel(world: &mut World) {
    let show_panel = world.resource::<UIMenu>().is_clicked("Engine/Entities");
    if !show_panel {
        return;
    }
    let selected = world.resource::<SelectedEntity>().0;
    let mut component_rows = Vec::<String>::new();
    let mut selected_label = None::<String>;
    let mut selected_alive = false;

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
            .filter_map(|component_id| components.get_name(*component_id).map(|n| n.to_string()))
            .collect();
        component_rows.sort();
    }

    if selected.is_some() && !selected_alive {
        world.resource_mut::<SelectedEntity>().0 = None;
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

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for component_name in &component_rows {
                            ui.label(component_name);
                        }
                    });
                } else {
                    ui.label("Selected entity no longer exists.");
                }
            } else {
                ui.label("Select an entity in the Active Entities panel.");
            }
        });
}
