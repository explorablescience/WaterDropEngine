use bevy::prelude::*;
use wde_gizmos::prelude::*;
use wde_renderer::prelude::Color;
use wde_editor::prelude::*;

use crate::navgrid::NavMap;

pub struct NavMapDebugPlugin;
impl Plugin for NavMapDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavMapDebugSettings>()
            .add_systems(Update, (manage_debug_settings, interior_show_nav_map, exterior_show_nav_map));
    }
}

#[derive(Resource, Default)]
struct NavMapDebugSettings {
    pub show_interior_nav_map: bool,
    pub show_exterior_nav_map: bool,
}

fn manage_debug_settings(
    ctx: Res<UIContext>,
    mut settings: ResMut<NavMapDebugSettings>,
    mut ui_menu: ResMut<UIMenu>
) {
    UIWindow::new("Navigation Debug Settings")
        .resizable(false)
        .default_pos([100.0, 260.0])
        .open(ui_menu.clicked_mut("Debug/Navigation"))
        .show(&ctx.0, |ui| {
            ui.add(Checkbox::new(
                &mut settings.show_interior_nav_map,
                "Show Interior Nav Map"
            ));
            ui.add(Checkbox::new(
                &mut settings.show_exterior_nav_map,
                "Show Exterior Nav Map"
            ));
        });
}


/// Draws every node and every connection exactly once.
fn interior_show_nav_map(nav_map: Res<NavMap>, mut gizmos: ResMut<Gizmos>, settings: Res<NavMapDebugSettings>) {
    // Only draw if interior nav map debug is enabled
    if !settings.show_interior_nav_map {
        return;
    }
    let nav_map = &nav_map.interior;
    for (index, node) in nav_map.nodes.iter().enumerate() {
        if !node.alive {
            continue;
        }
        gizmos.cube(
            Transform::from_xyz(node.position.x, 0.2, node.position.y).with_scale(Vec3::splat(0.1)),
            if node.connections.len() <= 1 {
                Color::GREEN
            } else {
                Color::RED
            },
        );

        for &other_id in &node.connections {
            if (other_id as usize) <= index {
                continue;
            }
            let other_node = &nav_map.nodes[other_id as usize];
            if !other_node.alive {
                continue;
            }
            let color = if node.connections.len() <= 1 || other_node.connections.len() <= 1 {
                Color::GREEN
            } else {
                Color::RED
            };
            gizmos.line(
                Vec3::new(node.position.x, 0.2, node.position.y),
                Vec3::new(other_node.position.x, 0.2, other_node.position.y),
                color,
            );
        }
    }
}

fn exterior_show_nav_map(nav_map: Res<NavMap>, mut gizmos: ResMut<Gizmos>, settings: Res<NavMapDebugSettings>) {
    // Only draw if exterior nav map debug is enabled
    if !settings.show_exterior_nav_map {
        return;
    }

    // TODO:
}
