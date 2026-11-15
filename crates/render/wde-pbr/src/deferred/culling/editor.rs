use bevy::prelude::*;
use wde_editor::prelude::*;
use wde_renderer::prelude::MAX_MESH_LODS;

use super::{CULL_VIEW_COUNT, CullingSettings, CullingStatsShared, LodSettings};

const VIEW_NAMES: [&str; CULL_VIEW_COUNT] = ["Camera", "Cascade 0", "Cascade 1", "Cascade 2"];
/// Same colors as `lod_debug_albedo` in the shaders.
const LOD_COLORS: [Color32; MAX_MESH_LODS] = [
    Color32::from_rgb(26, 204, 26),
    Color32::from_rgb(230, 217, 26),
    Color32::from_rgb(255, 115, 13),
    Color32::from_rgb(230, 13, 13)
];
const NO_LOD_COLOR: Color32 = Color32::from_rgb(51, 102, 242);

pub(crate) struct CullingEditorPlugin;
impl Plugin for CullingEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, edit_culling_settings.in_set(EngineUiSet));
    }
}

fn edit_culling_settings(
    ctx: Res<UIContext>,
    mut settings: ResMut<CullingSettings>,
    mut lod: ResMut<LodSettings>,
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
                    "{name}: {} drawn, {} groups culled, {}k triangles",
                    stats.drawn[view],
                    stats.groups_culled[view],
                    stats.triangles[view] / 1000
                ));
            }
            ui.separator();

            ui.columns(2, |cols| {
                cols[0].add(Checkbox::new(&mut lod.enabled, "Levels of Detail"));
                cols[1].add(Checkbox::new(&mut lod.show_levels, "Show Levels"));
            });
            ui.add(Slider::new(&mut lod.max_pixel_error, 0.1..=10.0).text("Max Pixel Error"));
            let mut forced = lod.force_level.is_some();
            let mut level = lod.force_level.unwrap_or(0);
            ui.columns(2, |cols| {
                cols[0].add(Checkbox::new(&mut forced, "Force Level"));
                cols[1].add(Slider::new(&mut level, 0..=MAX_MESH_LODS - 1));
            });
            lod.force_level = forced.then_some(level);
            ui.horizontal(|ui| {
                ui.label("Camera instances by level:");
                for (level, count) in stats.lods.iter().enumerate() {
                    ui.colored_label(LOD_COLORS[level], format!("{level}: {count}"));
                }
                ui.colored_label(NO_LOD_COLOR, format!("none: {}", stats.without_lods));
            });
        });
}
