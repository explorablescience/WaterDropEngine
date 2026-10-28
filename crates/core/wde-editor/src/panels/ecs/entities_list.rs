use bevy::{ecs::observer::Observer, prelude::*, window::Monitor};
use wde_egui::prelude::*;

use crate::{panels::ecs::entities_edit_components::SelectedEntity, ui::UIMenu};

type ListedFilter = (Without<Window>, Without<Monitor>, Without<Observer>);

struct Row {
    entity: Entity,
    depth: usize,
    has_children: bool,
    is_open: bool
}

pub fn draw_entities_panel(world: &mut World) {
    let mut panel_open = *world
        .resource_mut::<UIMenu>()
        .clicked_mut("Engine/Entities");
    if !panel_open {
        return;
    }

    let ctx = world.resource::<EguiContext>().0.clone();
    let mut selected = world.resource::<SelectedEntity>().0;

    let total = world
        .query_filtered::<(), ListedFilter>()
        .iter(world)
        .count();
    let mut roots = world
        .query_filtered::<Entity, (ListedFilter, Without<ChildOf>)>()
        .iter(world)
        .collect::<Vec<_>>();
    roots.sort_unstable_by_key(|entity| entity.index());
    let rows = flatten_visible_rows(world, &ctx, &roots);

    egui::Window::new("Entities")
        .default_size([500.0, 440.0])
        .open(&mut panel_open)
        .show(&ctx, |ui| {
            ui.label(format!("Total active entities: {}", total));
            if let Some(entity) = selected {
                ui.label(format!("Selected: {}", entity_label(world, entity)));
            }
            ui.separator();

            let row_height = ui.spacing().interact_size.y;
            let indent = ui.spacing().indent;
            egui::ScrollArea::vertical().auto_shrink(false).show_rows(
                ui,
                row_height,
                rows.len(),
                |ui, range| {
                    for row in &rows[range] {
                        ui.horizontal(|ui| {
                            ui.add_space(row.depth as f32 * indent);
                            if row.has_children {
                                let icon = if row.is_open { "▼" } else { "▶" };
                                if ui.add(egui::Button::new(icon).frame(false)).clicked() {
                                    ui.data_mut(|data| {
                                        data.insert_persisted(open_id(row.entity), !row.is_open)
                                    });
                                }
                            } else {
                                ui.add_space(indent);
                            }

                            let label = entity_label(world, row.entity);
                            if ui
                                .selectable_label(selected == Some(row.entity), label)
                                .clicked()
                            {
                                selected = Some(row.entity);
                            }
                        });
                    }
                }
            );
        });

    world.resource_mut::<SelectedEntity>().0 = selected;
    *world
        .resource_mut::<UIMenu>()
        .clicked_mut("Engine/Entities") = panel_open;
}

/// Depth-first list of the rows whose ancestors are all expanded.
fn flatten_visible_rows(world: &World, ctx: &egui::Context, roots: &[Entity]) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut stack = roots.iter().rev().map(|&e| (e, 0)).collect::<Vec<_>>();
    while let Some((entity, depth)) = stack.pop() {
        let children = world.get::<Children>(entity).filter(|c| !c.is_empty());
        let is_open = children.is_some()
            && ctx.data_mut(|data| {
                data.get_persisted::<bool>(open_id(entity))
                    .unwrap_or_default()
            });

        rows.push(Row {
            entity,
            depth,
            has_children: children.is_some(),
            is_open
        });

        if is_open && let Some(children) = children {
            stack.extend(children.iter().rev().map(|child| (child, depth + 1)));
        }
    }
    rows
}

fn open_id(entity: Entity) -> egui::Id {
    egui::Id::new(("entity_tree_open", entity))
}

fn entity_label(world: &World, entity: Entity) -> String {
    match world.get::<Name>(entity) {
        Some(name) => name.to_string(),
        None => format!("{:?}", entity)
    }
}
