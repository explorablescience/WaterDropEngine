use bevy::prelude::*;
use wde_egui::prelude::*;

use crate::ui::UIMenu;

#[derive(Resource, Default)]
pub struct SelectedEntity(pub Option<Entity>);

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
