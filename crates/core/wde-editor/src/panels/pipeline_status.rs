//! `Debug/Pipeline Status` window: every pipeline with its status and shaders, and the parts of passes skipped in
//! the last frame (see [`PipelineStatusView`]).
use bevy::prelude::*;
use wde_egui::prelude::*;
use wde_renderer::prelude::*;

use crate::prelude::*;

const FAILED: egui::Color32 = egui::Color32::from_rgb(220, 70, 70);
const PENDING: egui::Color32 = egui::Color32::from_rgb(240, 180, 50);
const READY: egui::Color32 = egui::Color32::from_rgb(110, 190, 110);

pub struct PipelineStatusPanelPlugin;
impl Plugin for PipelineStatusPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PipelineStatusPanelState>()
            .add_systems(Update, draw_ui.in_set(EngineUiSet).after(EditorMenuBarSet));
    }
}

#[derive(Resource, Default)]
struct PipelineStatusPanelState {
    filter: String,
    hide_ready: bool
}

fn draw_ui(
    ctx: Res<EguiContext>,
    mut ui_menu: ResMut<UIMenu>,
    mut view: ResMut<PipelineStatusView>,
    mut state: ResMut<PipelineStatusPanelState>
) {
    let open = ui_menu.clicked_mut("Debug/Pipeline Status");
    if view.enabled != *open {
        view.enabled = *open;
    }
    if !*open {
        return;
    }
    let view = &*view;
    egui::Window::new("Pipeline Status")
        .default_size([700.0, 550.0])
        .resizable(true)
        .open(open)
        .show(&ctx.0, |ui| {
            summary(ui, &view.pipelines);
            ui.separator();
            egui::CollapsingHeader::new(format!("Skipped last frame ({})", view.skips.len()))
                .default_open(true)
                .show(ui, |ui| skip_list(ui, view));
            ui.separator();
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut state.filter)
                        .hint_text("Filter by label or shader")
                );
                ui.checkbox(&mut state.hide_ready, "Hide ready");
            });
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| pipeline_list(ui, &view.pipelines, &state));
        });
}

fn summary(ui: &mut egui::Ui, pipelines: &[DebugPipelineReport]) {
    let count =
        |f: fn(&DebugPipelineStatus) -> bool| pipelines.iter().filter(|p| f(&p.status)).count();
    let failed = count(|s| matches!(s, DebugPipelineStatus::Failed(_)));
    let pending = count(|s| {
        matches!(
            s,
            DebugPipelineStatus::Pending | DebugPipelineStatus::WaitingLayouts
        )
    });
    let ready = count(|s| *s == DebugPipelineStatus::Ready);
    ui.horizontal(|ui| {
        ui.label(format!("{} pipelines:", pipelines.len()));
        ui.colored_label(READY, format!("{ready} ready"));
        ui.colored_label(
            if pending > 0 {
                PENDING
            } else {
                ui.visuals().weak_text_color()
            },
            format!("{pending} pending")
        );
        ui.colored_label(
            if failed > 0 {
                FAILED
            } else {
                ui.visuals().weak_text_color()
            },
            format!("{failed} failed")
        );
    });
}

fn skip_list(ui: &mut egui::Ui, view: &PipelineStatusView) {
    if view.skips.is_empty() {
        ui.weak("Nothing skipped.");
        return;
    }
    egui::Grid::new("pipeline-status-skips")
        .num_columns(4)
        .striped(true)
        .show(ui, |ui| {
            for column in ["Pass", "Sub-pass", "Reason", "Count"] {
                ui.weak(column);
            }
            ui.end_row();
            for skip in &view.skips {
                ui.label(format!("{} {}", skip.pass, skip.pass_label));
                ui.label(skip.sub_pass.unwrap_or("-"));
                skip_reason(ui, &skip.reason, &view.pipelines);
                ui.monospace(skip.count.to_string());
                ui.end_row();
            }
        });
}

fn skip_reason(ui: &mut egui::Ui, reason: &DebugSkipReason, pipelines: &[DebugPipelineReport]) {
    let DebugSkipReason::Pipeline(id) = reason else {
        ui.colored_label(PENDING, reason.to_string());
        return;
    };
    let Some(pipeline) = pipelines.iter().find(|p| p.id == Some(*id)) else {
        ui.colored_label(FAILED, format!("pipeline {id} not found"));
        return;
    };
    let (text, color) = status_label(&pipeline.status);
    let response = ui.colored_label(
        color,
        format!("pipeline '{}' {}", pipeline.label, text.to_lowercase())
    );
    if let DebugPipelineStatus::Failed(error) = &pipeline.status {
        response.on_hover_text(egui::RichText::new(error).monospace());
    }
}

fn status_label(status: &DebugPipelineStatus) -> (&'static str, egui::Color32) {
    match status {
        DebugPipelineStatus::Failed(_) => ("Failed", FAILED),
        DebugPipelineStatus::WaitingLayouts => ("Waiting", PENDING),
        DebugPipelineStatus::Pending => ("Pending", PENDING),
        DebugPipelineStatus::Ready => ("Ready", READY)
    }
}

fn pipeline_list(
    ui: &mut egui::Ui,
    pipelines: &[DebugPipelineReport],
    state: &PipelineStatusPanelState
) {
    if pipelines.is_empty() {
        ui.weak("Waiting for a frame...");
    }
    let filter = state.filter.to_lowercase();
    for pipeline in pipelines {
        if state.hide_ready && pipeline.status == DebugPipelineStatus::Ready {
            continue;
        }
        if !filter.is_empty()
            && !pipeline.label.to_lowercase().contains(&filter)
            && !pipeline
                .shaders
                .iter()
                .any(|s| s.path.to_lowercase().contains(&filter))
        {
            continue;
        }
        let (text, color) = status_label(&pipeline.status);
        let failed = matches!(pipeline.status, DebugPipelineStatus::Failed(_));
        let id = ui.make_persistent_id(("pipeline-status", pipeline.label, pipeline.id));
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, failed)
            .show_header(ui, |ui| {
                ui.add_sized(
                    [60.0, ui.available_height()],
                    egui::Label::new(egui::RichText::new(text).color(color).strong())
                );
                ui.label(pipeline.label);
                ui.weak(format!(
                    "{}{}",
                    if pipeline.compute {
                        "compute"
                    } else {
                        "render"
                    },
                    pipeline.id.map_or(String::new(), |id| format!(" #{id}"))
                ));
            })
            .body(|ui| pipeline_details(ui, pipeline));
    }
}

fn pipeline_details(ui: &mut egui::Ui, pipeline: &DebugPipelineReport) {
    match &pipeline.status {
        DebugPipelineStatus::Failed(error) => {
            ui.horizontal(|ui| {
                ui.colored_label(
                    FAILED,
                    "Failed to build. It is rebuilt when one of its shaders changes."
                );
                if ui.small_button("Copy").clicked() {
                    ui.ctx().copy_text(error.clone());
                }
            });
            egui::Frame::group(ui.style()).show(ui, |ui| {
                egui::ScrollArea::both()
                    .id_salt(("pipeline-status-error", pipeline.id))
                    .max_height(300.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(egui::RichText::new(error).monospace())
                                .wrap_mode(egui::TextWrapMode::Extend)
                        );
                    });
            });
        }
        DebugPipelineStatus::WaitingLayouts => {
            ui.colored_label(
                PENDING,
                "Not queued: one of its bind group layouts is not created yet."
            );
        }
        DebugPipelineStatus::Pending => {
            ui.colored_label(PENDING, "Waiting for its shaders to load.");
        }
        DebugPipelineStatus::Ready => {}
    }
    for shader in &pipeline.shaders {
        ui.horizontal(|ui| {
            match &shader.status {
                DebugShaderStatus::Loaded => ui.weak("loaded"),
                DebugShaderStatus::Loading => ui.colored_label(PENDING, "loading"),
                DebugShaderStatus::Failed(_) => ui.colored_label(FAILED, "failed")
            };
            ui.monospace(&shader.path);
        });
        if let DebugShaderStatus::Failed(error) = &shader.status {
            ui.colored_label(FAILED, egui::RichText::new(error).monospace());
        }
    }
}
