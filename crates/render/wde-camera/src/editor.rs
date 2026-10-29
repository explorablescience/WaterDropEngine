use crate::prelude::*;
use bevy::prelude::*;
use wde_editor::prelude::{ui::egui, *};

pub(crate) struct CameraPropertiesEditor;
impl Plugin for CameraPropertiesEditor {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (edit_view_params, edit_camera_roles).in_set(EngineUiSet)
        );
    }
}

fn edit_view_params(
    ctx: Res<UIContext>,
    mut camera_query: Query<&mut CameraView, With<ActiveCamera>>,
    mut ui_menu: ResMut<UIMenu>
) {
    if let Ok(mut view) = camera_query.single_mut() {
        UIWindow::new("Camera View Properties")
            .resizable(false)
            .default_pos([100.0, 10.0])
            .open(ui_menu.clicked_mut("Camera/View"))
            .show(&ctx.0, |ui| {
                ui.add(
                    Slider::new(&mut view.fov, 1.0..=179.0)
                        .text("FOV")
                        .suffix("°")
                );
                ui.add(
                    Slider::new(&mut view.znear, 0.01..=10.0)
                        .text("Near Plane")
                        .suffix(" units")
                );
                ui.add(
                    Slider::new(&mut view.zfar, 10.0..=10000.0)
                        .text("Far Plane")
                        .suffix(" units")
                );
            });
    }
}

#[allow(clippy::type_complexity)]
fn edit_camera_roles(
    mut commands: Commands,
    ctx: Res<UIContext>,
    mut ui_menu: ResMut<UIMenu>,
    cameras: Query<
        (
            Entity,
            Option<&Name>,
            Has<ActiveCamera>,
            Has<CullingCamera>,
            Has<ControlledCamera>
        ),
        With<Camera>
    >
) {
    UIWindow::new("Cameras")
        .resizable(false)
        .default_pos([100.0, 10.0])
        .open(ui_menu.clicked_mut("Camera/Cameras"))
        .show(&ctx.0, |ui| {
            egui::Grid::new("camera_roles")
                .striped(true)
                .show(ui, |ui| {
                    ui.label("");
                    for role in CameraRole::ALL {
                        ui.strong(role.label());
                    }
                    ui.end_row();
                    for (camera, name, rendered, culling, controlled) in &cameras {
                        ui.label(name.map_or_else(|| camera.to_string(), |n| n.to_string()));
                        for (role, held) in CameraRole::ALL
                            .into_iter()
                            .zip([rendered, culling, controlled])
                        {
                            if ui.radio(held, "").clicked() && !held {
                                commands.queue(SetCameraRole { role, camera });
                            }
                        }
                        ui.end_row();
                    }
                });
        });
}
