use bevy::prelude::*;
use wde_editor::prelude::*;
use wde_renderer::prelude::MAX_MESH_LODS;

use super::{CULL_VIEW_COUNT, CullingSettings, CullingStatsShared, LodSettings, MeshDebugView};

const VIEW_NAMES: [&str; CULL_VIEW_COUNT] = ["Camera", "Cascade 0", "Cascade 1", "Cascade 2"];
/// Same colors as `MESH_DEBUG_COLORS` in the shaders.
const LOD_COLORS: [Color32; MAX_MESH_LODS] = [
    Color32::from_rgb(26, 204, 26),
    Color32::from_rgb(230, 217, 26),
    Color32::from_rgb(255, 115, 13),
    Color32::from_rgb(230, 13, 13)
];
const NO_LOD_COLOR: Color32 = Color32::from_rgb(51, 102, 242);
const DENSITY_LEGEND: [(Color32, &str); 5] = [
    (LOD_COLORS[3], "≤1"),
    (LOD_COLORS[2], "4"),
    (LOD_COLORS[1], "16"),
    (LOD_COLORS[0], "64"),
    (NO_LOD_COLOR, "≥256")
];

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
    UIWindow::new("Culling")
        .resizable(false)
        .open(ui_menu.clicked_mut("Render/Culling"))
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

            ui.add(Checkbox::new(&mut lod.enabled, "Levels of Detail"));
            ui.horizontal(|ui| {
                ui.label("View:");
                ComboBox::from_id_salt("mesh_debug_view")
                    .selected_text(format!("{:?}", lod.debug_view))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut lod.debug_view, MeshDebugView::Off, "Off");
                        ui.selectable_value(&mut lod.debug_view, MeshDebugView::Levels, "Levels");
                        ui.selectable_value(
                            &mut lod.debug_view,
                            MeshDebugView::TriangleDensity,
                            "Triangle Density"
                        );
                    });
                ui.add(Checkbox::new(&mut lod.wireframe, "Wireframe"));
            });
            if lod.debug_view == MeshDebugView::TriangleDensity {
                ui.horizontal(|ui| {
                    ui.label("Pixels per triangle:");
                    for (color, label) in DENSITY_LEGEND {
                        ui.colored_label(color, label);
                    }
                });
            }
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
