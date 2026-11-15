use bevy::prelude::*;
use wde_editor::prelude::{ui::egui, *};

use super::settings::*;

pub(crate) struct PostProcessEditorPlugin;
impl Plugin for PostProcessEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, edit_post_process_settings.in_set(EngineUiSet));
    }
}

fn edit_post_process_settings(
    ctx: Res<UIContext>,
    mut settings: ResMut<PostProcessSettings>,
    mut ui_menu: ResMut<UIMenu>
) {
    UIWindow::new("Post Processing")
        .resizable(false)
        .default_pos([100.0, 260.0])
        .open(ui_menu.clicked_mut("PBR/Post Processing"))
        .show(&ctx.0, |ui| {
            let s = &mut *settings;

            egui::CollapsingHeader::new("Fog").show(ui, |ui| {
                let fog = &mut s.fog;
                ui.add(Checkbox::new(&mut fog.enabled, "Enabled"));
                ui.columns(2, |cols| {
                    cols[0].add(
                        Slider::new(&mut fog.density, 0.0..=0.05)
                            .logarithmic(true)
                            .text("Density")
                    );
                    cols[1].add(
                        Slider::new(&mut fog.height_falloff, 0.0..=1.0)
                            .logarithmic(true)
                            .text("Height Falloff")
                    );
                });
                ui.columns(2, |cols| {
                    cols[0]
                        .add(Slider::new(&mut fog.base_height, -100.0..=200.0).text("Base Height"));
                    cols[1].add(
                        Slider::new(&mut fog.start_distance, 0.0..=500.0).text("Start Distance")
                    );
                });
                ui.columns(2, |cols| {
                    cols[0].add(Slider::new(&mut fog.max_opacity, 0.0..=1.0).text("Max Opacity"));
                    cols[1]
                        .add(Slider::new(&mut fog.sky_distance, 0.0..=5000.0).text("Sky Distance"));
                });
                ui.columns(2, |cols| {
                    cols[0].add(Slider::new(&mut fog.sky_color, 0.0..=1.0).text("Sky Color"));
                    color(&mut cols[1], "Color", &mut fog.color);
                });
                ui.columns(2, |cols| {
                    cols[0]
                        .add(Slider::new(&mut fog.sun_intensity, 0.0..=2.0).text("Sun Intensity"));
                    cols[1]
                        .add(Slider::new(&mut fog.sun_exponent, 1.0..=64.0).text("Sun Exponent"));
                });
                color(ui, "Sun Color", &mut fog.sun_color);
            });

            egui::CollapsingHeader::new("Bloom").show(ui, |ui| {
                let bloom = &mut s.bloom;
                ui.add(Checkbox::new(&mut bloom.enabled, "Enabled"));
                ui.columns(2, |cols| {
                    cols[0].add(Slider::new(&mut bloom.intensity, 0.0..=1.0).text("Intensity"));
                    cols[1].add(Slider::new(&mut bloom.scatter, 0.0..=1.0).text("Scatter"));
                });
                ui.columns(2, |cols| {
                    cols[0].add(Slider::new(&mut bloom.threshold, 0.0..=4.0).text("Threshold"));
                    cols[1].add(
                        Slider::new(&mut bloom.threshold_softness, 0.0..=1.0).text("Softness")
                    );
                });
            });

            egui::CollapsingHeader::new("Color Grading").show(ui, |ui| {
                let grading = &mut s.color_grading;
                ui.columns(2, |cols| {
                    cols[0].add(Slider::new(&mut grading.exposure, -4.0..=4.0).text("Exposure"));
                    cols[1].add(Slider::new(&mut grading.contrast, 0.5..=2.0).text("Contrast"));
                });
                ui.columns(2, |cols| {
                    cols[0]
                        .add(Slider::new(&mut grading.temperature, -1.0..=1.0).text("Temperature"));
                    cols[1].add(Slider::new(&mut grading.tint, -1.0..=1.0).text("Tint"));
                });
                ui.columns(2, |cols| {
                    cols[0].add(Slider::new(&mut grading.saturation, 0.0..=2.0).text("Saturation"));
                    color(&mut cols[1], "Color Filter", &mut grading.color_filter);
                });
                ui.label("Lift / Gamma / Gain");
                for (label, value, range) in [
                    ("Lift", &mut grading.lift, -0.2..=0.2),
                    ("Gamma", &mut grading.gamma, 0.5..=2.0),
                    ("Gain", &mut grading.gain, 0.5..=2.0)
                ] {
                    ui.columns(3, |cols| {
                        for (i, channel) in ["R", "G", "B"].into_iter().enumerate() {
                            cols[i].add(
                                Slider::new(&mut value[i], range.clone())
                                    .text(format!("{label} {channel}"))
                            );
                        }
                    });
                }
            });

            egui::CollapsingHeader::new("Tonemapping").show(ui, |ui| {
                egui::ComboBox::from_label("Operator")
                    .selected_text(format!("{:?}", s.tonemapping))
                    .show_ui(ui, |ui| {
                        for tonemapping in [
                            Tonemapping::None,
                            Tonemapping::Reinhard,
                            Tonemapping::Aces,
                            Tonemapping::AgX,
                            Tonemapping::PbrNeutral
                        ] {
                            ui.selectable_value(
                                &mut s.tonemapping,
                                tonemapping,
                                format!("{tonemapping:?}")
                            );
                        }
                    });
            });

            egui::CollapsingHeader::new("Effects").show(ui, |ui| {
                let vignette = &mut s.vignette;
                ui.label("Vignette");
                ui.columns(2, |cols| {
                    cols[0].add(Slider::new(&mut vignette.intensity, 0.0..=1.0).text("Intensity"));
                    cols[1].add(Slider::new(&mut vignette.radius, 0.0..=1.5).text("Radius"));
                });
                ui.columns(2, |cols| {
                    cols[0]
                        .add(Slider::new(&mut vignette.smoothness, 0.01..=1.5).text("Smoothness"));
                    cols[1].add(Slider::new(&mut vignette.roundness, 0.0..=1.0).text("Roundness"));
                });
                color(ui, "Vignette Color", &mut vignette.color);

                ui.add(Separator::default());
                ui.columns(2, |cols| {
                    cols[0].add(
                        Slider::new(&mut s.chromatic_aberration.intensity, 0.0..=2.0)
                            .text("Chromatic Aberration")
                    );
                    cols[1].add(Checkbox::new(&mut s.dithering, "Dithering"));
                });
                ui.columns(2, |cols| {
                    cols[0].add(
                        Slider::new(&mut s.film_grain.intensity, 0.0..=1.0).text("Film Grain")
                    );
                    cols[1].add(
                        Slider::new(&mut s.film_grain.response, 0.0..=1.0).text("Grain Response")
                    );
                });
            });
        });
}

fn color(ui: &mut egui::Ui, label: &str, color: &mut [f32; 3]) {
    ui.horizontal(|ui| {
        ui.color_edit_button_rgb(color);
        ui.label(label);
    });
}
