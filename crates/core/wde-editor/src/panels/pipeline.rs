//! `Debug/Pipeline` window: the passes of the last frame with their timings, and the textures they write at each
//! of their points (see [`RenderGraphDebugView`]).
use bevy::prelude::*;
use wde_egui::prelude::*;
use wde_renderer::prelude::*;

use super::pipeline_plot::{PlotState, pass_color, swatch, timing_plot};
use crate::{prelude::*, ui_textures::UITextures};

/// Largest side of the texture the selected texture is drawn into.
const MAX_TARGET_SIZE: u32 = 2048;
/// Width of a timing column.
const TIMING_WIDTH: f32 = 48.0;

pub struct PipelinePanelPlugin;
impl Plugin for PipelinePanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PipelinePanelState>()
            .add_systems(Update, draw_ui.in_set(EngineUiSet).after(EditorMenuBarSet));
    }
}

#[derive(Resource, Default)]
struct PipelinePanelState {
    target: Option<(Handle<Texture>, (u32, u32))>,
    /// Label and format of the texture whose default view settings were applied.
    viewed: Option<(String, Option<TextureFormat>)>,
    fullscreen: bool,
    plot: PlotState
}

fn draw_ui(
    ctx: Res<EguiContext>,
    mut ui_menu: ResMut<UIMenu>,
    mut view: ResMut<RenderGraphDebugView>,
    mut state: ResMut<PipelinePanelState>,
    mut ui_textures: ResMut<UITextures>,
    asset_server: Res<AssetServer>
) {
    let open = ui_menu.clicked_mut("Debug/Pipeline");
    if view.enabled != *open {
        view.enabled = *open;
    }
    if !*open {
        return;
    }
    update_target(&mut view, &mut state, &mut ui_textures, &asset_server);
    let image = state
        .target
        .as_ref()
        .and_then(|(handle, size)| Some((ui_textures.get_texture_index(handle.id())?, *size)));

    let report = view.report.clone();
    let timings = view.timings.clone();
    egui::Window::new("Pipeline")
        .default_size([1100.0, 650.0])
        .resizable(true)
        .open(open)
        .show(&ctx.0, |ui| {
            egui::SidePanel::left("pipeline-passes")
                .resizable(true)
                .default_width(380.0)
                .show_inside(ui, |ui| {
                    egui::CollapsingHeader::new("Timings")
                        .default_open(true)
                        .show(ui, |ui| {
                            frame_timings(ui, &timings, &mut view.timing_window);
                            timing_plot(ui, &timings, &report, view.timing_window, &mut state.plot);
                        });
                    ui.separator();
                    timing_columns(ui, "Pass", "GPU", "CPU");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if let Some(selection) = pass_list(ui, &report, view.selection) {
                            view.selection = Some(selection);
                        }
                    });
                });
            egui::CentralPanel::default().show_inside(ui, |ui| {
                texture_view(ui, &report, &mut view, &mut state, image);
            });
            // The central panel does not count as used space, which would shrink the window to the side panel
            ui.expand_to_include_rect(ui.max_rect());
        });

    if state.fullscreen
        && let Some((id, _)) = image
    {
        // Below the menu bar
        let rect = ctx.0.available_rect();
        ctx.0.layer_painter(egui::LayerId::background()).image(
            id,
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE
        );
    }
}

/// Draws the passes and their points. Returns the clicked point.
fn pass_list(
    ui: &mut egui::Ui,
    report: &[DebugPassReport],
    selection: Option<DebugSelection>
) -> Option<DebugSelection> {
    if report.is_empty() {
        ui.weak("Waiting for a frame...");
    }
    let mut clicked = None;
    for pass in report {
        let in_pass = selection.filter(|s| s.pass == pass.id);
        let mut title = egui::RichText::new(if pass.id == RenderGraphDebugView::FINAL_PASS {
            "Final frame".to_string()
        } else {
            format!(
                "{} {}{}",
                pass.id,
                pass.label,
                if pass.custom { " (custom)" } else { "" }
            )
        });
        if in_pass.is_some() {
            title = title.strong();
        }
        let id = ui.make_persistent_id(("pipeline-pass", pass.id));
        let mut title_clicked = false;
        let mut header =
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
                .show_header(ui, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        timing_label(ui, pass.cpu_time, "Time to record the commands of the pass");
                        timing_label(ui, pass.gpu_time, "Time of the pass on the GPU");
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            if pass.id != RenderGraphDebugView::FINAL_PASS {
                                swatch(ui, pass_color(pass.id));
                            }
                            title_clicked = ui
                                .add(
                                    egui::Label::new(title)
                                        .sense(egui::Sense::click())
                                        .truncate()
                                )
                                .clicked();
                        });
                    });
                });
        if title_clicked {
            header.toggle();
        }
        header.body(|ui| {
            if pass.points.is_empty() {
                ui.weak("Nothing rendered this frame");
            }
            for (i, point) in pass.points.iter().enumerate() {
                let selected = in_pass.is_some_and(|s| s.point == i);
                if ui
                    .selectable_label(selected, format!("{i}: {}", point.label))
                    .clicked()
                {
                    // Keeps the texture of the previous point of this pass, if any
                    let texture = in_pass
                        .map(|s| s.texture)
                        .filter(|t| point.textures.get(*t).is_some_and(|t| t.viewable))
                        .or_else(|| point.textures.iter().position(|t| t.viewable))
                        .unwrap_or(0);
                    clicked = Some(DebugSelection {
                        pass: pass.id,
                        point: i,
                        texture
                    });
                }
            }
        });
    }
    clicked
}

/// Frame timings, averaged over the `window`, with their range.
fn frame_timings(ui: &mut egui::Ui, timings: &DebugFrameTimings, window: &mut usize) {
    egui::Grid::new("pipeline-timings")
        .num_columns(4)
        .striped(true)
        .show(ui, |ui| {
            ui.weak("ms");
            for column in ["avg", "min", "max"] {
                ui.weak(column);
            }
            ui.end_row();
            let gpu = if timings.gpu_supported {
                "Render graph on the GPU, from the start of its first pass to the end of its last"
            } else {
                "The device does not support timestamp queries"
            };
            for (label, timing, hover) in [
                (
                    "Frame",
                    timings.frame,
                    "Interval between two presented frames, every wait included"
                ),
                (
                    "Render",
                    timings.render,
                    "Render thread, waits for the swapchain (present mode, GPU) included"
                ),
                (
                    "Render busy",
                    timings.render_busy,
                    "Render thread, waits for the swapchain excluded"
                ),
                (
                    "Swapchain wait",
                    timings.swapchain_wait,
                    "Time blocked acquiring and presenting the swapchain texture, waiting for the present mode \
                     (vsync) or the GPU"
                ),
                (
                    "Graph CPU",
                    timings.graph_cpu,
                    "Recording of the render graph commands"
                ),
                ("Graph GPU", timings.graph_gpu, gpu)
            ] {
                ui.label(label).on_hover_text(hover);
                match timing {
                    Some(t) => {
                        ui.monospace(format!("{:.2}", t.avg));
                        ui.monospace(format!("{:.2}", t.min));
                        ui.monospace(format!("{:.2}", t.max));
                    }
                    None => {
                        for _ in 0..3 {
                            ui.weak("-");
                        }
                    }
                }
                ui.end_row();
            }
        });
    ui.horizontal(|ui| {
        if let Some(frame) = timings.frame.filter(|f| f.avg > 0.0) {
            ui.strong(format!("{:.0} fps", 1000.0 / frame.avg));
        }
        if let Some(mode) = timings.present_mode {
            ui.weak(format!("{mode:?}"));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(
                egui::DragValue::new(window)
                    .range(1..=1000)
                    .suffix(" frames")
            )
            .on_hover_text("Number of frames the timings are computed over");
        });
    });
}

/// Header of the pass list, aligned with its timings.
fn timing_columns(ui: &mut egui::Ui, title: &str, gpu: &str, cpu: &str) {
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for column in [cpu, gpu] {
                ui.add_sized(
                    [TIMING_WIDTH, ui.available_height()],
                    egui::Label::new(egui::RichText::new(column).weak())
                );
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.weak(title);
            });
        });
    });
}

/// Average of a timing, in milliseconds, with its range on hover.
fn timing_label(ui: &mut egui::Ui, timing: Option<DebugTiming>, hover: &str) {
    let text = timing.map_or("-".to_string(), |t| format!("{:.2}", t.avg));
    let response = ui.add_sized(
        [TIMING_WIDTH, ui.available_height()],
        egui::Label::new(egui::RichText::new(text).monospace())
    );
    match timing {
        Some(t) => response.on_hover_text(format!(
            "{hover}, in ms.\navg {:.3}, min {:.3}, max {:.3}",
            t.avg, t.min, t.max
        )),
        None => response.on_hover_text(hover)
    };
}

fn texture_view(
    ui: &mut egui::Ui,
    report: &[DebugPassReport],
    view: &mut RenderGraphDebugView,
    state: &mut PipelinePanelState,
    image: Option<(egui::TextureId, (u32, u32))>
) {
    let Some(selection) = view.selection else {
        ui.label(
            "Select a point of a pass, to view the textures it wrote at that point of the frame."
        );
        return;
    };
    let Some(point) = report
        .iter()
        .find(|p| p.id == selection.pass)
        .and_then(|p| p.points.get(selection.point))
    else {
        ui.label("The selected point was not reached in the last frame.");
        return;
    };

    // Textures of the point
    ui.horizontal_wrapped(|ui| {
        for (i, texture) in point.textures.iter().enumerate() {
            ui.add_enabled_ui(texture.viewable, |ui| {
                if ui
                    .selectable_label(i == selection.texture, &texture.label)
                    .clicked()
                    && let Some(selection) = view.selection.as_mut()
                {
                    selection.texture = i;
                }
            });
        }
    });
    let Some(texture) = point.textures.get(selection.texture) else {
        return;
    };
    match texture.format {
        Some(format) => ui.weak(format!(
            "{format:?}, {}x{}, {} layer(s), {} mip(s)",
            texture.size.0, texture.size.1, texture.layers, texture.mips
        )),
        None => ui.weak("Not viewable")
    };
    ui.separator();

    // View settings
    let settings = &mut view.settings;
    ui.horizontal(|ui| {
        if let Some(format) = texture
            .format
            .filter(|f| f.is_combined_depth_stencil_format())
        {
            ui.label("Aspect");
            let mut stencil = settings.stencil;
            egui::ComboBox::from_id_salt("pipeline-aspect")
                .selected_text(if stencil { "Stencil" } else { "Depth" })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut stencil, false, "Depth");
                    ui.selectable_value(&mut stencil, true, "Stencil");
                });
            if stencil != settings.stencil {
                *settings = view_settings(format, stencil);
            }
        }
        ui.label("Mode");
        egui::ComboBox::from_id_salt("pipeline-mode")
            .selected_text(format!("{:?}", settings.mode))
            .show_ui(ui, |ui| {
                for mode in [
                    DebugViewMode::Data,
                    DebugViewMode::Color,
                    DebugViewMode::Integer
                ] {
                    ui.selectable_value(&mut settings.mode, mode, format!("{mode:?}"));
                }
            });
        ui.label("Channels");
        egui::ComboBox::from_id_salt("pipeline-channels")
            .selected_text(format!("{:?}", settings.channels))
            .show_ui(ui, |ui| {
                for channels in [
                    DebugChannels::Rgb,
                    DebugChannels::R,
                    DebugChannels::G,
                    DebugChannels::B,
                    DebugChannels::A
                ] {
                    ui.selectable_value(&mut settings.channels, channels, format!("{channels:?}"));
                }
            });
    });
    ui.horizontal_wrapped(|ui| {
        if settings.mode == DebugViewMode::Integer {
            integer_legend(ui);
            return;
        }
        let [min, max] = &mut settings.range;
        let speed = ((*max - *min).abs() * 0.002).max(1e-5);
        ui.label("Range");
        ui.add(egui::DragValue::new(min).speed(speed).max_decimals(5));
        ui.add(egui::DragValue::new(max).speed(speed).max_decimals(5));
        if settings.mode == DebugViewMode::Color {
            ui.add(egui::Slider::new(&mut settings.exposure, -10.0..=10.0).text("Exposure (EV)"));
            ui.checkbox(&mut settings.tonemap, "Tonemap");
        }
    });
    ui.horizontal(|ui| {
        if texture.layers > 1 {
            ui.add(egui::Slider::new(&mut settings.layer, 0..=texture.layers - 1).text("Layer"));
        }
        if texture.mips > 1 {
            ui.add(egui::Slider::new(&mut settings.mip, 0..=texture.mips - 1).text("Mip"));
        }
        ui.checkbox(&mut state.fullscreen, "Fullscreen");
        if ui.button("Reset view").clicked()
            && let Some(format) = texture.format
        {
            *settings = view_settings(format, settings.stencil);
        }
    });
    ui.weak("NaN in magenta, infinity in cyan.");

    // Image, fitted to the panel
    let Some((id, size)) = image.filter(|_| texture.viewable) else {
        return;
    };
    let available = ui.available_size();
    let aspect = size.0 as f32 / size.1.max(1) as f32;
    let width = available.x.min(available.y * aspect).max(1.0);
    let response = ui.image(egui::load::SizedTexture::new(
        id,
        egui::vec2(width, width / aspect)
    ));
    if let Some(pos) = response.hover_pos() {
        let uv = (pos - response.rect.min) / response.rect.size();
        let mip_size = mip_size(texture.size, view.settings.mip);
        ui.weak(format!(
            "Texel ({}, {})",
            (uv.x * mip_size.0 as f32) as u32,
            (uv.y * mip_size.1 as f32) as u32
        ));
    }
}

fn view_settings(format: TextureFormat, stencil: bool) -> DebugViewSettings {
    if stencil {
        DebugViewSettings::for_stencil()
    } else {
        DebugViewSettings::for_format(format)
    }
}

/// Colors of the first values of the [`DebugViewMode::Integer`].
fn integer_legend(ui: &mut egui::Ui) {
    ui.label("Values");
    for value in 0..16 {
        let [r, g, b] = debug_integer_color(value);
        swatch(ui, egui::Rgba::from_rgb(r, g, b).into());
        ui.monospace(value.to_string());
    }
    ui.weak("...");
}

/// Recreates the texture the selected texture is drawn into when its size changes, and applies the default view
/// settings of a newly selected texture.
fn update_target(
    view: &mut RenderGraphDebugView,
    state: &mut PipelinePanelState,
    ui_textures: &mut UITextures,
    asset_server: &AssetServer
) {
    let Some(texture) = view.selected_texture().filter(|t| t.viewable).cloned() else {
        return;
    };
    let viewed = (texture.label.clone(), texture.format);
    if state.viewed.as_ref() != Some(&viewed) {
        if let Some(format) = texture.format {
            view.settings = DebugViewSettings::for_format(format);
        }
        state.viewed = Some(viewed);
    }

    let (width, height) = mip_size(texture.size, view.settings.mip);
    let scale = (MAX_TARGET_SIZE as f32 / width.max(height) as f32).min(1.0);
    let size = (
        ((width as f32 * scale) as u32).max(1),
        ((height as f32 * scale) as u32).max(1)
    );
    if state.target.as_ref().is_some_and(|(_, s)| *s == size) {
        return;
    }
    let handle = asset_server.add(Texture {
        label: "debug-view".to_string(),
        size,
        format: RenderGraphDebugView::TARGET_FORMAT,
        usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        ..Default::default()
    });
    ui_textures.register_texture_weak(handle.id());
    view.target = Some(handle.id());
    state.target = Some((handle, size));
}

fn mip_size(size: (u32, u32), mip: u32) -> (u32, u32) {
    ((size.0 >> mip).max(1), (size.1 >> mip).max(1))
}
