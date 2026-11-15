//! Plot of the pass timings of the last frames, stacked, in the `Debug/Pipeline` window.
use wde_egui::prelude::*;
use wde_renderer::prelude::*;

const HEIGHT: f32 = 140.0;
/// Frame budgets drawn as dashed lines.
const TARGETS: [(u32, f32); 2] = [(60, 1000.0 / 60.0), (30, 1000.0 / 30.0)];

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum PlotSource {
    #[default]
    Gpu,
    Cpu
}

#[derive(Default)]
pub(super) struct PlotState {
    source: PlotSource,
    /// Top of the vertical axis in ms, eased down to not jump with each spike.
    scale: f32
}

/// Distinct color of a pass, stable across frames.
pub(super) fn pass_color(id: RenderPassId) -> egui::Color32 {
    let hue = (id as f32 * 0.618_034).rem_euclid(1.0);
    let value = if id.rem_euclid(2) == 0 { 0.95 } else { 0.7 };
    egui::ecolor::Hsva::new(hue, 0.7, value, 1.0).into()
}

pub(super) fn swatch(ui: &mut egui::Ui, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 2.0, color);
}

/// Times of the passes over the last `window` frames, stacked in their execution order, with the frame interval
/// as a line and the 60 and 30 fps budgets dashed.
pub(super) fn timing_plot(
    ui: &mut egui::Ui,
    timings: &DebugFrameTimings,
    report: &[DebugPassReport],
    window: usize,
    state: &mut PlotState
) {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.source, PlotSource::Gpu, "GPU");
        ui.selectable_value(&mut state.source, PlotSource::Cpu, "CPU");
        ui.weak("passes stacked, frame interval as a line");
    });
    let history = match state.source {
        PlotSource::Gpu => &timings.gpu_history,
        PlotSource::Cpu => &timings.cpu_history
    };
    let columns = window.max(2);

    // Bands in the execution order of the last frame, then the passes it skipped
    let mut order: Vec<RenderPassId> = report
        .iter()
        .map(|p| p.id)
        .filter(|id| *id != RenderGraphDebugView::FINAL_PASS)
        .collect();
    for (id, _) in history.iter().flatten() {
        if !order.contains(id) {
            order.push(*id);
        }
    }

    // Top of each band, for the frames shown, the newest in the last column
    let stacks: Vec<Vec<f32>> = history
        .iter()
        .skip(history.len().saturating_sub(columns))
        .map(|frame| {
            let mut top = 0.0;
            order
                .iter()
                .map(|id| {
                    top += frame
                        .iter()
                        .find(|(p, _)| p == id)
                        .map_or(0.0, |(_, ms)| *ms);
                    top
                })
                .collect()
        })
        .collect();
    let first = columns - stacks.len();
    let frames: Vec<f32> = timings
        .frame_history
        .iter()
        .skip(timings.frame_history.len().saturating_sub(columns))
        .copied()
        .collect();
    let first_frame = columns - frames.len();

    let peak = stacks
        .iter()
        .filter_map(|s| s.last())
        .chain(&frames)
        .fold(0.0_f32, |a, b| a.max(*b));
    let target = (peak * 1.1).max(TARGETS[0].1 * 1.25);
    state.scale = if target > state.scale {
        target
    } else {
        state.scale + (target - state.scale) * 0.05
    };
    let scale = state.scale;

    let (response, painter) = ui.allocate_painter(
        egui::vec2(ui.available_width(), HEIGHT),
        egui::Sense::hover()
    );
    let rect = response.rect;
    let visuals = ui.visuals();
    let x = |column: usize| rect.left() + column as f32 * rect.width() / (columns - 1) as f32;
    let y = |ms: f32| rect.bottom() - (ms / scale).min(1.0) * rect.height();
    painter.rect_filled(rect, 2.0, visuals.extreme_bg_color);

    // Grid
    let step = nice_step(scale / 4.0);
    let decimals = (-step.log10().floor()).max(0.0) as usize;
    let grid = egui::Stroke::new(1.0, visuals.weak_text_color().gamma_multiply(0.25));
    for i in 1.. {
        let ms = step * i as f32;
        if ms >= scale * 0.92 {
            break;
        }
        painter.hline(rect.x_range(), y(ms), grid);
        painter.text(
            egui::pos2(rect.left() + 3.0, y(ms)),
            egui::Align2::LEFT_BOTTOM,
            format!("{ms:.decimals$} ms"),
            egui::FontId::monospace(9.0),
            visuals.weak_text_color()
        );
    }

    // Bands, as quads between consecutive frames
    let mut mesh = egui::Mesh::default();
    for (k, id) in order.iter().enumerate() {
        let color = pass_color(*id);
        let base = mesh.vertices.len() as u32;
        for (i, stack) in stacks.iter().enumerate() {
            let bottom = if k == 0 { 0.0 } else { stack[k - 1] };
            mesh.colored_vertex(egui::pos2(x(first + i), y(bottom)), color);
            mesh.colored_vertex(egui::pos2(x(first + i), y(stack[k])), color);
        }
        for i in 0..stacks.len().saturating_sub(1) as u32 {
            let a = base + 2 * i;
            mesh.add_triangle(a, a + 1, a + 3);
            mesh.add_triangle(a, a + 3, a + 2);
        }
    }
    painter.add(mesh);

    // Frame interval, under the budgets
    let frame_line: Vec<egui::Pos2> = frames
        .iter()
        .enumerate()
        .map(|(i, ms)| egui::pos2(x(first_frame + i), y(*ms)))
        .collect();
    painter.add(egui::Shape::line(
        frame_line,
        egui::Stroke::new(1.0, visuals.strong_text_color())
    ));
    for (i, (fps, ms)) in TARGETS.into_iter().enumerate() {
        if ms >= scale {
            continue;
        }
        let color = if i == 0 {
            visuals.warn_fg_color
        } else {
            visuals.weak_text_color()
        };
        painter.extend(egui::Shape::dashed_line(
            &[
                egui::pos2(rect.left(), y(ms)),
                egui::pos2(rect.right(), y(ms))
            ],
            egui::Stroke::new(1.0, color),
            5.0,
            4.0
        ));
        painter.text(
            egui::pos2(rect.right() - 3.0, y(ms)),
            egui::Align2::RIGHT_BOTTOM,
            format!("{fps} fps"),
            egui::FontId::proportional(10.0),
            color
        );
    }

    // Hovered frame and pass
    let Some(pos) = response.hover_pos() else {
        return;
    };
    let column = (((pos.x - rect.left()) / rect.width()) * (columns - 1) as f32).round() as usize;
    painter.vline(
        x(column),
        rect.y_range(),
        egui::Stroke::new(1.0, visuals.weak_text_color())
    );
    let stack = column.checked_sub(first).and_then(|i| stacks.get(i));
    let hovered_ms = (rect.bottom() - pos.y) / rect.height() * scale;
    let band = stack.and_then(|s| s.iter().position(|top| hovered_ms <= *top));
    if let Some(k) = band {
        let outline = |level: &dyn Fn(&Vec<f32>) -> f32| {
            stacks
                .iter()
                .enumerate()
                .map(|(i, s)| egui::pos2(x(first + i), y(level(s))))
                .collect::<Vec<_>>()
        };
        let stroke = egui::Stroke::new(1.5, visuals.strong_text_color());
        painter.add(egui::Shape::line(outline(&|s| s[k]), stroke));
        if k > 0 {
            painter.add(egui::Shape::line(outline(&|s| s[k - 1]), stroke));
        }
    }
    let frame = column.checked_sub(first_frame).and_then(|i| frames.get(i));
    response.on_hover_ui_at_pointer(|ui| {
        ui.weak(format!("{} frame(s) ago", columns - 1 - column));
        if let (Some(k), Some(stack)) = (band, stack) {
            let id = order[k];
            let label = report.iter().find(|p| p.id == id).map_or("", |p| p.label);
            ui.horizontal(|ui| {
                swatch(ui, pass_color(id));
                ui.strong(format!(
                    "{id} {label}: {:.3} ms",
                    stack[k] - if k == 0 { 0.0 } else { stack[k - 1] }
                ));
            });
        }
        if let Some(total) = stack.and_then(|s| s.last()) {
            ui.label(format!("Passes: {total:.3} ms"));
        }
        if let Some(frame) = frame {
            ui.label(format!("Frame: {frame:.3} ms"));
        }
    });
}

/// Smallest 1, 2 or 5 times a power of ten above `raw`.
fn nice_step(raw: f32) -> f32 {
    let power = 10f32.powf(raw.max(1e-3).log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|m| m * power)
        .find(|step| *step >= raw)
        .unwrap_or(10.0 * power)
}
