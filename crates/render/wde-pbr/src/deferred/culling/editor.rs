use bevy::prelude::*;
use wde_editor::prelude::*;

use super::{CULL_VIEW_COUNT, CullingSettings, CullingStatsShared};

const VIEW_NAMES: [&str; CULL_VIEW_COUNT] = ["Camera", "Cascade 0", "Cascade 1", "Cascade 2"];

pub(crate) struct CullingEditorPlugin;
impl Plugin for CullingEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, edit_culling_settings.in_set(EngineUiSet));
    }
}

fn edit_culling_settings(
    ctx: Res<UIContext>,
    mut settings: ResMut<CullingSettings>,
    stats: Res<CullingStatsShared>,
    mut ui_menu: ResMut<UIMenu>
) {
    let stats = stats.0.lock().map(|s| *s).unwrap_or_default();
    UIWindow::new("PBR Culling")
        .resizable(false)
        .default_pos([100.0, 620.0])
        .open(ui_menu.clicked_mut("PBR/Culling"))
        .show(&ctx.0, |ui| {
            ui.columns(2, |cols| {
                cols[0].add(Checkbox::new(&mut settings.enabled, "Enabled"));
                cols[1].add(Checkbox::new(&mut settings.shadows, "Shadow Casters"));
            });
            ui.columns(2, |cols| {
                cols[0].add(Checkbox::new(&mut settings.groups, "Groups"));
                cols[1].add(Checkbox::new(&mut settings.meshes, "Meshes"));
            });
            ui.add(Checkbox::new(&mut settings.show_groups, "Show Groups"));
            ui.separator();

            ui.label(format!(
                "{} instances, {} groups, {} mesh tests, {:.0} µs",
                stats.instances, stats.groups, stats.mesh_tests, stats.micros
            ));
            for (view, name) in VIEW_NAMES.iter().enumerate() {
                ui.label(format!(
                    "{name}: {} drawn, {} groups culled",
                    stats.drawn[view], stats.groups_culled[view]
                ));
            }
        });
}
