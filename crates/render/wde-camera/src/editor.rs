use crate::prelude::*;
use bevy::prelude::*;
use wde_editor::prelude::*;

pub(crate) struct CameraPropertiesEditor;
impl Plugin for CameraPropertiesEditor {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, edit_view_params.in_set(EngineUiSet));
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
