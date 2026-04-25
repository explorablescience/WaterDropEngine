use bevy::prelude::*;
use wde_editor::prelude::*;

use crate::prelude::AtmosphereSettings;

pub(crate) struct AtmosphereEditorPlugin;
impl Plugin for AtmosphereEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, edit_atmosphere_settings);
    }
}

fn edit_atmosphere_settings(
    ctx: Res<UIContext>,
    mut atmosphere: ResMut<AtmosphereSettings>,
    mut ui_menu: ResMut<UIMenu>
) {
    UIWindow::new("Atmosphere")
        .resizable(false)
        .default_pos([100.0, 260.0])
        .open(ui_menu.clicked_mut("PBR/Atmosphere"))
        .show(&ctx.0, |ui| {
            ui.add(Checkbox::new(&mut atmosphere.animate_time, "Animate Time"));
            ui.columns(2, |cols| {
                cols[0].add(Slider::new(&mut atmosphere.time_of_day, 0.0..=1.0).text("Time Of Day"));
                cols[1].add(
                    Slider::new(&mut atmosphere.day_length_seconds, 10.0..=1200.0)
                        .text("Day Length")
                        .suffix(" s")
                );
            });
            ui.columns(2, |cols| {
                cols[0].add(Slider::new(&mut atmosphere.sun_intensity, 0.0..=3.0).text("Sun Intensity"));
                cols[1].add(
                    Slider::new(&mut atmosphere.twilight_softness, 0.0..=1.0)
                        .text("Twilight Softness")
                );
            });
            ui.columns(2, |cols| {
                cols[0].add(Slider::new(&mut atmosphere.star_intensity, 0.0..=2.0).text("Star Intensity"));
            });

            ui.add(Separator::default());
            ui.label("Gradient");
            ui.columns(2, |cols| {
                cols[0].add(
                    Slider::new(&mut atmosphere.day_horizon_exp, 0.1..=3.0)
                        .text("Day Horizon Pow")
                );
                cols[1].add(
                    Slider::new(&mut atmosphere.night_horizon_exp, 0.1..=3.0)
                        .text("Night Horizon Pow")
                );
            });
            ui.columns(2, |cols| {
                cols[0].add(
                    Slider::new(&mut atmosphere.twilight_horizon_exp, 0.1..=3.0)
                        .text("Twilight Horizon Pow")
                );
                cols[1].add(
                    Slider::new(&mut atmosphere.twilight_intensity, 0.0..=2.0)
                        .text("Twilight Intensity")
                );
            });

            ui.add(Separator::default());
            ui.label("Sun");
            ui.columns(2, |cols| {
                cols[0].add(
                    Slider::new(&mut atmosphere.sun_visibility_min, -1.0..=1.0)
                        .text("Visibility Min")
                );
                cols[1].add(
                    Slider::new(&mut atmosphere.sun_visibility_max, -1.0..=1.0)
                        .text("Visibility Max")
                );
            });
            ui.columns(2, |cols| {
                cols[0].add(Slider::new(&mut atmosphere.sun_disk_min, 0.95..=1.0).text("Disk Min"));
                cols[1].add(Slider::new(&mut atmosphere.sun_disk_max, 0.95..=1.0).text("Disk Max"));
            });
            ui.columns(2, |cols| {
                cols[0].add(
                    Slider::new(&mut atmosphere.sun_corona_exp, 1.0..=128.0)
                        .text("Corona Exponent")
                );
                cols[1].add(
                    Slider::new(&mut atmosphere.sun_corona_intensity, 0.0..=4.0)
                        .text("Corona Intensity")
                );
            });

            ui.add(Separator::default());
            ui.label("Stars");
            ui.columns(2, |cols| {
                cols[0].add(
                    Slider::new(&mut atmosphere.star_density_threshold, 0.95..=0.99999)
                        .text("Density Threshold")
                );
                cols[1].add(
                    Slider::new(&mut atmosphere.star_core_scale, 50.0..=5000.0)
                        .text("Core Scale")
                );
            });
            ui.columns(2, |cols| {
                cols[0].add(
                    Slider::new(&mut atmosphere.star_min_altitude, -1.0..=1.0)
                        .text("Min Altitude")
                );
                cols[1].add(
                    Slider::new(&mut atmosphere.star_max_altitude, -1.0..=1.0)
                        .text("Max Altitude")
                );
            });

            ui.add(Separator::default());
            ui.label("Ground");
            ui.columns(2, |cols| {
                cols[0].add(Slider::new(&mut atmosphere.ground_falloff, 0.1..=10.0).text("Ground Falloff"));
            });

            ui.add(Separator::default());
            ui.label("Colors (RGB)");
            ui.columns(2, |cols| {
                color_rgb(&mut cols[0], "Day Zenith", &mut atmosphere.day_zenith_color);
                color_rgb(&mut cols[1], "Day Horizon", &mut atmosphere.day_horizon_color);
            });
            ui.columns(2, |cols| {
                color_rgb(&mut cols[0], "Night Zenith", &mut atmosphere.night_zenith_color);
                color_rgb(&mut cols[1], "Night Horizon", &mut atmosphere.night_horizon_color);
            });
            ui.columns(2, |cols| {
                color_rgb(&mut cols[0], "Twilight Tint", &mut atmosphere.twilight_tint_color);
                color_rgb(&mut cols[1], "Sun Day", &mut atmosphere.sun_day_color);
            });
            ui.columns(2, |cols| {
                color_rgb(&mut cols[0], "Sun Night", &mut atmosphere.sun_night_color);
                color_rgb(&mut cols[1], "Star", &mut atmosphere.star_color);
            });
            ui.columns(2, |cols| {
                color_rgb(&mut cols[0], "Ground Night", &mut atmosphere.ground_night_color);
                color_rgb(&mut cols[1], "Ground Day", &mut atmosphere.ground_day_color);
            });
        });
}

fn color_rgb(ui: &mut wde_editor::prelude::ui::egui::Ui, label: &str, color: &mut [f32; 3]) {
    let mut egui_color = Color32::from_rgb(
        (color[0] * 255.0) as u8,
        (color[1] * 255.0) as u8,
        (color[2] * 255.0) as u8
    );
    ui.label(label);
    if ui.color_edit_button_srgba(&mut egui_color).changed() {
        color[0] = egui_color.r() as f32 / 255.0;
        color[1] = egui_color.g() as f32 / 255.0;
        color[2] = egui_color.b() as f32 / 255.0;
    }
}
