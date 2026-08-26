use bevy::prelude::*;
use wde_editor::prelude::*;

use crate::prelude::PbrSettings;

pub(crate) struct PbrLightingEditorPlugin;
impl Plugin for PbrLightingEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, edit_pbr_settings.in_set(EngineUiSet));
    }
}

fn edit_pbr_settings(
    ctx: Res<UIContext>,
    mut pbr: ResMut<PbrSettings>,
    mut ui_menu: ResMut<UIMenu>
) {
    UIWindow::new("PBR Lighting")
        .resizable(false)
        .default_pos([100.0, 620.0])
        .open(ui_menu.clicked_mut("PBR/Lighting"))
        .show(&ctx.0, |ui| {
            ui.columns(2, |cols| {
                cols[0].add(Slider::new(&mut pbr.direct_scale, 0.0..=4.0).text("Direct Scale"));
                cols[1].add(Slider::new(&mut pbr.ambient_scale, 0.0..=2.0).text("Ambient Scale"));
            });
            ui.columns(2, |cols| {
                cols[0].add(Slider::new(&mut pbr.exposure, 0.0..=4.0).text("Exposure"));
                cols[1].add(Slider::new(&mut pbr.tone_map_white, 0.01..=4.0).text("ToneMap White"));
            });
            ui.columns(2, |cols| {
                cols[0].add(Slider::new(&mut pbr.metallic_scale, 0.0..=2.0).text("Metallic Scale"));
                cols[1]
                    .add(Slider::new(&mut pbr.roughness_scale, 0.0..=2.0).text("Roughness Scale"));
            });
            ui.columns(2, |cols| {
                cols[0].add(Slider::new(&mut pbr.min_roughness, 0.0..=1.0).text("Min Roughness"));
            });
        });
}
