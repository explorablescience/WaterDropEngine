use wde::prelude::*;

use bevy::prelude::*;

use crate::{base::*, selection::*};

pub struct SelectionDebugPlugin;
impl Plugin for SelectionDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, debug_selected);
    }
}

fn debug_selected(
    ctx: Res<UIContext>,
    mut ui_menu: ResMut<UIMenu>,
    selected: Res<SelectionState>,
    bases: Query<(&Base, &BaseElectricity)>,
    placed: Query<(
        &GridEntity,
        Option<&InBase>,
        Option<&ElectricityProducer>,
        Option<&ElectricityConsumer>
    )>,
    names: Query<&Name>
) {
    UIWindow::new("Selected Infos")
        .default_pos([100.0, 260.0])
        .default_width(420.0)
        .open(ui_menu.clicked_mut_default_open("Debug/Selection"))
        .show(&ctx.0, |ui| {
            if selected.selected.is_empty() {
                ui.label("No entity selected");
                return;
            }
            for (index, selected) in selected.selected.iter().enumerate() {
                if index > 0 {
                    ui.separator();
                }
                match selected {
                    EntitySelectionType::Building(entity) | EntitySelectionType::Module(entity) => {
                        let Ok((grid_entity, in_base, producer, consumer)) = placed.get(*entity)
                        else {
                            continue;
                        };
                        ui.strong(&grid_entity.entry().label);

                        match in_base.and_then(|b| bases.get(b.0).ok()) {
                            Some((base, electricity)) => {
                                ui.label(format!("Base: {}", base.name));
                                ui.label(format!(
                                    " - Electricity: {:+.0} ({:.0} produced, {:.0} consumed)",
                                    electricity.balance(),
                                    electricity.production,
                                    electricity.consumption
                                ));
                            }
                            None => {
                                ui.label("Base: none (not working)");
                            }
                        }

                        if producer.is_some() || consumer.is_some() {
                            ui.label("Electricity:");
                        }
                        if let Some(producer) = producer {
                            ui.label(format!(" - Producer: {:.0} units", producer.0));
                        }
                        if let Some(consumer) = consumer {
                            ui.label(format!(" - Consumer: {:.0} units", consumer.0));
                        }
                    }
                    EntitySelectionType::Entity(entity) => {
                        match names.get(*entity) {
                            Ok(name) => ui.strong(name.as_str()),
                            Err(_) => ui.strong(format!("{entity}"))
                        };
                    }
                }
            }
        });
}
