use bevy::prelude::*;
use wde_camera::prelude::*;
use wde_editor::prelude::*;

use crate::prelude::ThirdPersonController;

pub struct CameraPropertiesEditor;
impl Plugin for CameraPropertiesEditor {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, edit_controller_params);
    }
}

fn edit_controller_params(
    ctx: Res<UIContext>,
    mut camera_query: Query<&mut ThirdPersonController, With<ActiveCamera>>,
    mut ui_menu: ResMut<UIMenu>
) {
    if let Ok(mut controller) = camera_query.single_mut() {
        UIWindow::new("Camera Controller Properties")
            .resizable(false)
            .default_pos([100.0, 10.0])
            .open(ui_menu.clicked_mut("Camera/Controller"))
            .show(&ctx.0, |ui| {
                ui.add(Slider::new(&mut controller.sensitivity, 0.001..=1.0).text("Sensitivity"));
                ui.add(Slider::new(&mut controller.friction, 0.0..=1.0).text("Friction"));
                ui.add(
                    Slider::new(&mut controller.min_move_speed, 0.1..=5000.0)
                        .text("Min Move Speed")
                );
                ui.add(
                    Slider::new(&mut controller.max_move_speed, 0.1..=5000.0)
                        .text("Max Move Speed")
                );
                ui.add(
                    Slider::new(&mut controller.run_speed_multiplier, 1.0..=10.0)
                        .text("Run Speed Multiplier")
                );
                ui.add(Slider::new(&mut controller.zoom_speed, 0.1..=100.0).text("Zoom Speed"));
                ui.add(Slider::new(&mut controller.distance, 1.0..=1000.0).text("Distance"));
                ui.add(
                    Slider::new(&mut controller.min_distance, 0.1..=1000.0).text("Min Distance")
                );
                ui.add(
                    Slider::new(&mut controller.max_distance, 1.0..=1000.0).text("Max Distance")
                );
                ui.add(
                    Slider::new(&mut controller.min_pitch, 0.0..=std::f32::consts::FRAC_PI_2)
                        .text("Min Pitch")
                );
                ui.add(
                    Slider::new(&mut controller.max_pitch, 0.0..=std::f32::consts::FRAC_PI_2)
                        .text("Max Pitch")
                );
                ui.add(Checkbox::new(
                    &mut controller.edge_scroll_enabled,
                    "Enable Edge Scrolling"
                ));
                ui.add(
                    Slider::new(&mut controller.edge_scroll_distance, 0.0..=200.0)
                        .text("Edge Scroll Distance")
                );
                ui.add(
                    Slider::new(&mut controller.edge_scroll_speed, 0.0..=1000.0)
                        .text("Edge Scroll Speed")
                );

                // Reset button
                if ui.button("Reset").clicked() {
                    *controller = ThirdPersonController::default();
                }
            });
    }
}
