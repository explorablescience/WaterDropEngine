use bevy::prelude::*;
use wde_editor::prelude::*;

use crate::prelude::{PbrDebugView, PbrSettings};

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
    UIWindow::new("Lighting")
        .resizable(false)
        .open(ui_menu.clicked_mut("Render/Lighting"))
        .show(&ctx.0, |ui| {
            ui.columns(2, |cols| {
                cols[0].add(Slider::new(&mut pbr.direct_scale, 0.0..=4.0).text("Direct Scale"));
                cols[1].add(Slider::new(&mut pbr.ambient_scale, 0.0..=2.0).text("Ambient Scale"));
            });
            ui.columns(2, |cols| {
                cols[0].add(Slider::new(&mut pbr.exposure, 0.0..=4.0).text("Exposure"));
                cols[1].add(Slider::new(&mut pbr.min_roughness, 0.0..=1.0).text("Min Roughness"));
            });
            ui.columns(2, |cols| {
                cols[0].add(Slider::new(&mut pbr.metallic_scale, 0.0..=2.0).text("Metallic Scale"));
                cols[1]
                    .add(Slider::new(&mut pbr.roughness_scale, 0.0..=2.0).text("Roughness Scale"));
            });
            ui.separator();

            let mut view = pbr.debug_view;
            ComboBox::from_label("Debug View")
                .selected_text(view.label())
                .show_ui(ui, |ui| {
                    for (i, option) in PbrDebugView::ALL.into_iter().enumerate() {
                        if i == 1
                            || matches!(option, PbrDebugView::ShadowCascades | PbrDebugView::Albedo)
                        {
                            ui.separator();
                        }
                        ui.selectable_value(&mut view, option, option.label());
                    }
                });
            if view != pbr.debug_view {
                pbr.debug_view = view;
            }
            match pbr.debug_view {
                PbrDebugView::Off => {}
                v if v.is_data() => {
                    ui.label("Values as they are, without post-processing.");
                }
                _ => {
                    ui.label("Exposure frozen. No fog, bloom or screen effects.");
                }
            }
        });
}
