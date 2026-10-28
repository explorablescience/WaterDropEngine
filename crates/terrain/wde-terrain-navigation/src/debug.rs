use bevy::prelude::*;
use wde_editor::prelude::{ui::egui, *};
use wde_gizmos::prelude::*;
use wde_renderer::prelude::WdeColor;
use wde_terrain_grid::prelude::{BuildingGraph, GridEntity, LinkKind};

use crate::navigation::{
    NavPath,
    walkmap::{CELL_SIZE, WalkMap, is_airlock, is_pressurized}
};

/// Height of the debug graph above the ground.
const GRAPH_HEIGHT: f32 = 1.2;
/// Colors of the groups of linked buildings, cycled through.
const GROUP_COLORS: [WdeColor; 6] = [
    WdeColor::LinearRgba(0.3, 0.9, 0.4, 1.0),
    WdeColor::LinearRgba(0.3, 0.6, 1.0, 1.0),
    WdeColor::LinearRgba(1.0, 0.55, 0.0, 1.0),
    WdeColor::LinearRgba(0.85, 0.35, 1.0, 1.0),
    WdeColor::LinearRgba(1.0, 0.9, 0.2, 1.0),
    WdeColor::LinearRgba(1.0, 0.35, 0.35, 1.0)
];

pub struct NavDebugPlugin;
impl Plugin for NavDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavDebugSettings>()
            .register_type::<NavDebugSettings>()
            .add_systems(Update, (debug_ui, show_graph, show_paths, show_walk_cells));
    }
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
struct NavDebugSettings {
    show_graph: bool,
    show_paths: bool,
    show_walk_cells: bool,
    list_network_pieces: bool
}

fn debug_ui(
    ctx: Res<UIContext>,
    mut settings: ResMut<NavDebugSettings>,
    mut ui_menu: ResMut<UIMenu>,
    graph: Res<BuildingGraph>,
    buildings: Query<&GridEntity>
) {
    UIWindow::new("Navigation")
        .default_pos([100.0, 260.0])
        .default_width(420.0)
        .open(ui_menu.clicked_mut("Debug/Navigation"))
        .show(&ctx.0, |ui| {
            ui.horizontal(|ui| {
                ui.add(Checkbox::new(
                    &mut settings.show_graph,
                    "Show building graph"
                ));
                ui.add(Checkbox::new(&mut settings.show_paths, "Show paths"));
                ui.add(Checkbox::new(
                    &mut settings.show_walk_cells,
                    "Show walkable cells"
                ));
                ui.add(Checkbox::new(
                    &mut settings.list_network_pieces,
                    "List network pieces"
                ));
            });
            ui.separator();
            ui.label(format!(
                "{} building(s)  •  {} linked group(s)",
                graph.buildings().count(),
                graph.groups().len()
            ));
            ui.separator();

            // Groups of linked buildings, down to each building's links
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    for (index, group) in graph.groups().iter().enumerate() {
                        egui::CollapsingHeader::new(format!(
                            "Group #{}: {} building(s)",
                            index + 1,
                            group.len()
                        ))
                        .id_salt(("group", index))
                        .show(ui, |ui| {
                            building_list(ui, &graph, &buildings, group, &settings)
                        });
                    }
                });
        });
}

/// Lists buildings (network pieces last) with their links.
fn building_list(
    ui: &mut egui::Ui,
    graph: &BuildingGraph,
    buildings: &Query<&GridEntity>,
    entities: &[Entity],
    settings: &NavDebugSettings
) {
    let mut sorted: Vec<(Entity, &GridEntity)> = entities
        .iter()
        .filter_map(|e| buildings.get(*e).ok().map(|b| (*e, b)))
        .collect();
    sorted.sort_by_key(|(_, b)| (b.entry().network.is_some(), b.entry().label.clone()));
    let hidden = sorted
        .iter()
        .filter(|(_, b)| b.entry().network.is_some())
        .count();
    for (entity, building) in sorted {
        if building.entry().network.is_some() && !settings.list_network_pieces {
            continue;
        }
        let links = graph.links(entity);
        egui::CollapsingHeader::new(format!(
            "{} at {}  ({} link{})",
            building.entry().label,
            fmt_pos(building.center()),
            links.len(),
            if links.len() == 1 { "" } else { "s" }
        ))
        .id_salt(entity)
        .show(ui, |ui| {
            if links.is_empty() {
                ui.label(egui::RichText::new("No link").weak());
            }
            for link in links {
                let Ok(other) = buildings.get(link.entity) else {
                    continue;
                };
                ui.label(format!(
                    "{} {} → {} at {} ({})",
                    match link.kind {
                        LinkKind::Tunnel => "Tunnel",
                        LinkKind::Path => "Path"
                    },
                    link.anchor,
                    other.entry().label,
                    fmt_pos(other.center()),
                    link.neighbor_anchor
                ));
            }
        });
    }
    if hidden > 0 && !settings.list_network_pieces {
        ui.label(egui::RichText::new(format!("+ {hidden} network pieces")).weak());
    }
}

fn fmt_pos(pos: Vec2) -> String {
    format!("({:.0}, {:.0})", pos.x, pos.y)
}

/// Draws buildings, colored by group, and their links (cyan: tunnel, yellow: path).
fn show_graph(
    graph: Res<BuildingGraph>,
    buildings: Query<&GridEntity>,
    mut gizmos: ResMut<Gizmos>,
    settings: Res<NavDebugSettings>
) {
    if !settings.show_graph {
        return;
    }
    let at = |p: Vec2| Vec3::new(p.x, GRAPH_HEIGHT, p.y);
    for (index, group) in graph.groups().iter().enumerate() {
        let color = GROUP_COLORS[index % GROUP_COLORS.len()];
        for &entity in group {
            let Ok(building) = buildings.get(entity) else {
                continue;
            };
            gizmos.cube(
                Transform::from_translation(at(building.center())).with_scale(Vec3::splat(0.2)),
                color
            );
            for link in graph.links(entity) {
                // Each link is listed by both buildings: draw it once
                if entity > link.entity {
                    continue;
                }
                let Ok(other) = buildings.get(link.entity) else {
                    continue;
                };
                let color = match link.kind {
                    LinkKind::Tunnel => WdeColor::LinearRgba(0.2, 0.8, 1.0, 1.0),
                    LinkKind::Path => WdeColor::LinearRgba(1.0, 0.8, 0.1, 1.0)
                };
                gizmos.line(at(building.center()), at(link.position), color);
                gizmos.line(at(link.position), at(other.center()), color);
            }
        }
    }
}

/// Draws the remaining path of every moving entity.
fn show_paths(
    moving: Query<(&Transform, &NavPath)>,
    mut gizmos: ResMut<Gizmos>,
    settings: Res<NavDebugSettings>
) {
    if !settings.show_paths {
        return;
    }
    let lift = Vec3::Y * 0.05;
    let color = WdeColor::LinearRgba(1.0, 0.3, 0.9, 1.0);
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

/// Draws the walkable cells of buildings (cyan: pressurized, orange: airlock, green: outdoor).
fn show_walk_cells(
    walk: Res<WalkMap>,
    buildings: Query<&GridEntity>,
    mut gizmos: ResMut<Gizmos>,
    settings: Res<NavDebugSettings>
) {
    if !settings.show_walk_cells {
        return;
    }
    let half = CELL_SIZE * 0.35;
    for (key, cell) in &walk.cells {
        let color = match buildings.get(cell.building) {
            Ok(b) if is_airlock(b) => WdeColor::LinearRgba(1.0, 0.55, 0.0, 1.0),
            Ok(b) if is_pressurized(b) => WdeColor::LinearRgba(0.2, 0.8, 1.0, 1.0),
            _ => WdeColor::GREEN
        };
        let c = WalkMap::cell_center(*key);
        let y = cell.floor + 0.03;
        gizmos.line(
            Vec3::new(c.x - half, y, c.y - half),
            Vec3::new(c.x + half, y, c.y + half),
            color
        );
        gizmos.line(
            Vec3::new(c.x - half, y, c.y + half),
            Vec3::new(c.x + half, y, c.y - half),
            color
        );
    }
}
