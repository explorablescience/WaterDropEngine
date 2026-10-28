use wde::{prelude::*, wde_editor::prelude::ui::egui, wde_terrain_grid::core::grid::TILE_SIZE};

use bevy::prelude::*;

use crate::base::{Base, BaseElectricity, BaseMembers, InBase};

pub struct BaseDebugPlugin;
impl Plugin for BaseDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, debug_ui);
    }
}

fn debug_ui(
    ctx: Res<UIContext>,
    mut ui_menu: ResMut<UIMenu>,
    graph: Res<BuildingGraph>,
    buildings: Query<&GridEntity>,
    bases: Query<(&Base, &BaseElectricity, Option<&BaseMembers>)>,
    working: Query<(), With<InBase>>
) {
    UIWindow::new("Bases")
        .default_pos([100.0, 260.0])
        .default_width(420.0)
        .open(ui_menu.clicked_mut("Debug/Bases"))
        .show(&ctx.0, |ui| {
            let not_working = graph.buildings().filter(|e| !working.contains(*e)).count();
            ui.label(format!(
                "{} base(s)  •  {} building(s)  •  {} not working",
                bases.iter().len(),
                graph.buildings().count(),
                not_working
            ));
            ui.add_space(4.0);

            let problems = problems(&graph, &buildings);
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

            for (index, (base, electricity, members)) in bases.iter().enumerate() {
                let Ok(room) = buildings.get(base.control_room) else {
                    continue;
                };
                let members: Vec<&GridEntity> = members
                    .map(|m| buildings.iter_many(m.iter()).collect())
                    .unwrap_or_default();
                egui::CollapsingHeader::new(format!(
                    "Base #{}: control room at {}, {} members",
                    index + 1,
                    fmt_pos(room.center()),
                    members.len()
                ))
                .id_salt(("base", index))
                .show(ui, |ui| {
                    let color = match electricity.is_sufficient() {
                        true => egui::Color32::from_rgb(110, 200, 120),
                        false => egui::Color32::from_rgb(235, 110, 90)
                    };
                    ui.colored_label(
                        color,
                        format!(
                            "Electricity: {:+.0} ({:.0} produced, {:.0} consumed)",
                            electricity.balance(),
                            electricity.production,
                            electricity.consumption
                        )
                    );
                    ui.label(summarize(&members));
                });
            }
        });
}

/// Human readable description of every problem preventing buildings from working.
fn problems(graph: &BuildingGraph, buildings: &Query<&GridEntity>) -> Vec<String> {
    let mut problems = Vec::new();
    for group in graph.groups() {
        let members: Vec<(Entity, &GridEntity)> = group
            .iter()
            .filter_map(|e| buildings.get(*e).ok().map(|b| (*e, b)))
            .collect();
        let rooms: Vec<&GridEntity> = members
            .iter()
            .map(|(_, b)| *b)
            .filter(|b| b.entry().role.as_deref() == Some("control_room"))
            .collect();
        match rooms.len() {
            1 => continue,
            0 => {}
            n => {
                let positions: Vec<String> = rooms.iter().map(|b| fmt_pos(b.center())).collect();
                problems.push(format!(
                    "A base has {n} control rooms (at {}). Only one is allowed per base, so its {} \
                     buildings do not work: disconnect or remove the extra control rooms.",
                    positions.join(", "),
                    members.len()
                ));
                continue;
            }
        }

        // No control room
        let Some((_, first)) = members.first() else {
            continue;
        };
        let entry = first.entry();
        if members.len() == 1 && entry.anchors_in.is_empty() && entry.anchors_out.is_empty() {
            problems.push(format!(
                "{} at {} has no connection point, so it can never join a base.",
                entry.label,
                fmt_pos(first.center())
            ));
            continue;
        }
        let center = members.iter().map(|(_, b)| b.center()).sum::<Vec2>() / members.len() as f32;
        let them = if members.len() == 1 { "it" } else { "them" };
        let hint = match members
            .iter()
            .any(|(_, b)| !b.entry().anchors_in.is_empty())
        {
            true => format!("Link {them} with tunnels to a control room."),
            false => format!("Link {them} with paths to an airlock leading to a control room.")
        };
        let group: Vec<&GridEntity> = members.iter().map(|(_, b)| *b).collect();
        problems.push(format!(
            "{} not linked to any control room around {}, not working: {}. {}",
            match members.len() {
                1 => "1 building is".to_string(),
                n => format!("{n} buildings are")
            },
            fmt_pos(center),
            summarize(&group),
            hint
        ));

        // Common mistake: a path or tunnel passing along a building without ending on it
        for (entity, building) in &members {
            if building.entry().network.is_some() {
                continue;
            }
            let alongside = graph.buildings().find_map(|other| {
                let piece = buildings.get(other).ok()?;
                (piece.entry().network.is_some()
                    && touches(footprint_rect(building), footprint_rect(piece))
                    && !graph.links(*entity).iter().any(|l| l.entity == other))
                .then_some(piece)
            });
            if let Some(piece) = alongside {
                problems.push(format!(
                    "A {} runs alongside {} at {} without ending on it: lines only connect where \
                     they end against a building side.",
                    piece.entry().label.to_lowercase(),
                    building.entry().label,
                    fmt_pos(building.center())
                ));
            }
        }
    }
    problems
}

/// World XZ rectangle covered by the footprint (min, max).
fn footprint_rect(building: &GridEntity) -> (Vec2, Vec2) {
    let (a, b) = building.bbox();
    let half = Vec2::splat(TILE_SIZE / 2.0);
    (a.min(b) - half, a.max(b) + half)
}

/// Whether two rectangles share a side, without overlapping.
fn touches(a: (Vec2, Vec2), b: (Vec2, Vec2)) -> bool {
    let overlap = a.1.min(b.1) - a.0.max(b.0);
    let eps = 0.05;
    (overlap.x.abs() < eps && overlap.y > eps) || (overlap.y.abs() < eps && overlap.x > eps)
}

fn fmt_pos(pos: Vec2) -> String {
    format!("({:.0}, {:.0})", pos.x, pos.y)
}

/// Labels with their counts, e.g. `Dome Small, 3 × Residence`.
fn summarize(buildings: &[&GridEntity]) -> String {
    let mut counts: Vec<(&str, usize)> = Vec::new();
    for b in buildings {
        let label = b.entry().label.as_str();
        match counts.iter_mut().find(|(l, _)| *l == label) {
            Some((_, n)) => *n += 1,
            None => counts.push((label, 1))
        }
    }
    counts
        .iter()
        .map(|(label, n)| match n {
            1 => label.to_string(),
            n => format!("{n} × {label}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}
