//! `Debug/Pipeline` window: the passes of the last frame, and the textures they write at each of their points
//! (see [`RenderGraphDebugView`]).
use bevy::prelude::*;
use wde_egui::prelude::*;
use wde_renderer::prelude::*;

use crate::{prelude::*, ui_textures::UITextures};

/// Largest side of the texture the selected texture is drawn into.
const MAX_TARGET_SIZE: u32 = 2048;

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
    fullscreen: bool
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
    egui::Window::new("Pipeline")
        .default_size([1100.0, 650.0])
        .resizable(true)
        .open(open)
        .show(&ctx.0, |ui| {
            egui::SidePanel::left("pipeline-passes")
                .resizable(true)
                .default_width(260.0)
                .show_inside(ui, |ui| {
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
        egui::CollapsingHeader::new(title)
            .id_salt(pass.id)
            .show(ui, |ui| {
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
        ui.label("Mode");
        egui::ComboBox::from_id_salt("pipeline-mode")
            .selected_text(format!("{:?}", settings.mode))
            .show_ui(ui, |ui| {
                for mode in [DebugViewMode::Data, DebugViewMode::Color] {
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
    ui.horizontal(|ui| {
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
            *settings = DebugViewSettings::for_format(format);
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
