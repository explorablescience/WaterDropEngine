use bevy::prelude::*;
use wde_editor::prelude::{ui::egui, *};
use wde_gizmos::prelude::*;
use wde_renderer::prelude::Color;
use wde_terrain_grid::prelude::LinkKind;

use crate::{
    navgrid::graph::{NavBuilding, NavGrid, fmt_pos},
    navigation::NavPath
};

/// Height of the debug graph above the ground.
const GRAPH_HEIGHT: f32 = 1.2;

pub struct NavMapDebugPlugin;
impl Plugin for NavMapDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavMapDebugSettings>()
            .add_systems(Update, (debug_ui, show_graph, show_paths));
    }
}

#[derive(Resource, Default)]
struct NavMapDebugSettings {
    show_graph: bool,
    show_paths: bool,
    list_network_pieces: bool
}

fn debug_ui(
    ctx: Res<UIContext>,
    mut settings: ResMut<NavMapDebugSettings>,
    mut ui_menu: ResMut<UIMenu>,
    nav: Res<NavGrid>
) {
    UIWindow::new("Navigation")
        .default_pos([100.0, 260.0])
        .default_width(420.0)
        .open(ui_menu.clicked_mut("Debug/Navigation"))
        .show(&ctx.0, |ui| {
            ui.horizontal(|ui| {
                ui.add(Checkbox::new(
                    &mut settings.show_graph,
                    "Show network graph"
                ));
                ui.add(Checkbox::new(
                    &mut settings.show_paths,
                    "Show colonist paths"
                ));
                ui.add(Checkbox::new(
                    &mut settings.list_network_pieces,
                    "List tunnels and paths"
                ));
            });
            ui.separator();

            // Summary
            let not_working = nav
                .buildings
                .keys()
                .filter(|e| !nav.is_working(**e))
                .count();
            ui.label(format!(
                "{} base(s)  •  {} building(s)  •  {} not working",
                nav.bases.len(),
                nav.buildings.len(),
                not_working
            ));
            ui.add_space(4.0);

            // Problems
            let problems = nav.problems();
            if problems.is_empty() {
                ui.colored_label(
                    egui::Color32::from_rgb(110, 200, 120),
                    "No problems: every building is linked to a single control room."
                );
            } else {
                ui.strong(format!("Problems ({})", problems.len()));
                for problem in problems {
                    ui.colored_label(
                        egui::Color32::from_rgb(235, 110, 90),
                        format!("• {problem}")
                    );
                }
            }
            ui.separator();

            // Bases and unconnected groups, down to each building's neighbors
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    egui::CollapsingHeader::new(format!("Bases ({})", nav.bases.len()))
                        .default_open(true)
                        .show(ui, |ui| {
                            for (index, base) in nav.bases.iter().enumerate() {
                                let room = &nav.buildings[&base.control_rooms[0]];
                                let status = match base.is_valid() {
                                    true => String::new(),
                                    false => {
                                        format!("  ⚠ {} control rooms", base.control_rooms.len())
                                    }
                                };
                                egui::CollapsingHeader::new(format!(
                                    "Base #{}: control room at {}, {} buildings{}",
                                    index + 1,
                                    fmt_pos(room.center),
                                    base.buildings.len(),
                                    status
                                ))
                                .id_salt(("base", index))
                                .show(ui, |ui| building_list(ui, &nav, &base.buildings, &settings));
                            }
                        });
                    egui::CollapsingHeader::new(format!(
                        "Not linked to a base ({})",
                        nav.unconnected.iter().map(Vec::len).sum::<usize>()
                    ))
                    .show(ui, |ui| {
                        for (index, group) in nav.unconnected.iter().enumerate() {
                            egui::CollapsingHeader::new(format!(
                                "Group #{}: {} building(s)",
                                index + 1,
                                group.len()
                            ))
                            .id_salt(("unconnected", index))
                            .show(ui, |ui| building_list(ui, &nav, group, &settings));
                        }
                    });
                });
        });
}

/// Lists buildings (control rooms first, network pieces last) with their neighbors.
fn building_list(
    ui: &mut egui::Ui,
    nav: &NavGrid,
    entities: &[Entity],
    settings: &NavMapDebugSettings
) {
    let mut sorted: Vec<(&Entity, &NavBuilding)> =
        entities.iter().map(|e| (e, &nav.buildings[e])).collect();
    sorted.sort_by_key(|(_, b)| (!b.is_control_room, b.is_network, b.label.clone()));
    let hidden = sorted.iter().filter(|(_, b)| b.is_network).count();
    for (entity, building) in sorted {
        if building.is_network && !settings.list_network_pieces {
            continue;
        }
        egui::CollapsingHeader::new(format!(
            "{} at {}  ({} link{})",
            building.label,
            fmt_pos(building.center),
            building.neighbors.len(),
            if building.neighbors.len() == 1 {
                ""
            } else {
                "s"
            }
        ))
        .id_salt(entity)
        .show(ui, |ui| {
            if building.neighbors.is_empty() {
                ui.label(egui::RichText::new("No neighbor").weak());
            }
            for neighbor in &building.neighbors {
                let other = &nav.buildings[&neighbor.entity];
                ui.label(format!(
                    "{} {} → {} at {} ({})",
                    match neighbor.kind {
                        LinkKind::Tunnel => "Tunnel",
                        LinkKind::Path => "Path"
                    },
                    neighbor.anchor,
                    other.label,
                    fmt_pos(other.center),
                    neighbor.neighbor_anchor
                ));
            }
        });
    }
    if hidden > 0 && !settings.list_network_pieces {
        ui.label(egui::RichText::new(format!("+ {hidden} tunnel / path pieces")).weak());
    }
}

/// Draws buildings (green: working, orange: conflicting base, red: unconnected) and their links
/// (cyan: tunnel, yellow: path).
fn show_graph(nav: Res<NavGrid>, mut gizmos: ResMut<Gizmos>, settings: Res<NavMapDebugSettings>) {
    if !settings.show_graph {
        return;
    }
    let at = |p: Vec2| Vec3::new(p.x, GRAPH_HEIGHT, p.y);
    for (entity, building) in &nav.buildings {
        let color = match building.base.map(|b| nav.bases[b].is_valid()) {
            Some(true) => Color::GREEN,
            Some(false) => Color::LinearRgba(1.0, 0.55, 0.0, 1.0),
            None => Color::RED
        };
        let size = if building.is_control_room { 0.45 } else { 0.18 };
        gizmos.cube(
            Transform::from_translation(at(building.center)).with_scale(Vec3::splat(size)),
            color
        );
        for neighbor in &building.neighbors {
            // Each link is listed by both buildings: draw it once
            if *entity > neighbor.entity {
                continue;
            }
            let color = match neighbor.kind {
                LinkKind::Tunnel => Color::LinearRgba(0.2, 0.8, 1.0, 1.0),
                LinkKind::Path => Color::LinearRgba(1.0, 0.8, 0.1, 1.0)
            };
            let other = &nav.buildings[&neighbor.entity];
            gizmos.line(at(building.center), at(neighbor.position), color);
            gizmos.line(at(neighbor.position), at(other.center), color);
        }
    }
}

/// Draws the remaining path of every moving entity.
fn show_paths(
    moving: Query<(&Transform, &NavPath)>,
    mut gizmos: ResMut<Gizmos>,
    settings: Res<NavMapDebugSettings>
) {
    if !settings.show_paths {
        return;
    }
    let lift = Vec3::Y * 0.05;
    let color = Color::LinearRgba(1.0, 0.3, 0.9, 1.0);
    for (transform, path) in &moving {
        let mut from = transform.translation;
        for waypoint in &path.waypoints {
            gizmos.line(from + lift, waypoint.position + lift, color);
            from = waypoint.position;
        }
        gizmos.cube(
            Transform::from_translation(path.destination + lift).with_scale(Vec3::splat(0.12)),
            color
        );
    }
}
