use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use wde_egui::prelude::*;

const LONG_AVG_PERIOD_SECONDS: f64 = 5.0;

#[derive(Resource, Default)]
struct FrameDataAverages {
    fps_long_avg: Option<f64>,
    frame_ms_long_avg: Option<f64>,
}

// Display frame timing and other per-frame data in the editor UI. This is a separate plugin to avoid adding the overhead of these systems when not in editor mode.
pub struct UIFrameDataPlugin;
impl Plugin for UIFrameDataPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .init_resource::<FrameDataAverages>()
            .add_systems(
                Update,
                (update_long_averages, draw_framedata_overlay).chain(),
            );
    }
}

fn update_long_averages(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    mut averages: ResMut<FrameDataAverages>,
) {
    // Exponential moving average with a long time constant for stable debug trends.
    let alpha = (time.delta_secs_f64() / LONG_AVG_PERIOD_SECONDS).clamp(0.0, 1.0);

    if let Some(fps) = read_diagnostic(&diagnostics, FrameTimeDiagnosticsPlugin::FPS) {
        averages.fps_long_avg = Some(match averages.fps_long_avg {
            Some(prev) => prev + alpha * (fps - prev),
            None => fps,
        });
    }

    if let Some(frame_ms) = read_diagnostic(&diagnostics, FrameTimeDiagnosticsPlugin::FRAME_TIME) {
        averages.frame_ms_long_avg = Some(match averages.frame_ms_long_avg {
            Some(prev) => prev + alpha * (frame_ms - prev),
            None => frame_ms,
        });
    }
}

fn draw_framedata_overlay(
    ctx: Res<EguiContext>,
    diagnostics: Res<DiagnosticsStore>,
    averages: Res<FrameDataAverages>,
    window: Query<&Window, With<PrimaryWindow>>
) {
    let Some(window) = window.single().ok() else {
        return; // No primary window, can't draw overlay
    };

    let fps = read_diagnostic(&diagnostics, FrameTimeDiagnosticsPlugin::FPS);
    let frame_ms = read_diagnostic(&diagnostics, FrameTimeDiagnosticsPlugin::FRAME_TIME);
    let frame_count = read_diagnostic(&diagnostics, FrameTimeDiagnosticsPlugin::FRAME_COUNT);

    let text_color = egui::Color32::from_gray(165);
    let shadow_color = egui::Color32::from_black_alpha(220);
    let mut lines = Vec::with_capacity(6);
    lines.push(match frame_count {
        Some(value) => format!("Frame Count: {:.0}", value),
        None => "Frame Count: n/a".to_string(),
    });
    lines.push(match (fps, averages.fps_long_avg) {
        (Some(current), Some(long_avg)) => format!("FPS: {current:.1} (Avg: {long_avg:.1})"),
        _ => "FPS: n/a".to_string(),
    });
    lines.push(match (frame_ms, averages.frame_ms_long_avg) {
        (Some(current), Some(long_avg)) => format!("Frame Time: {current:.2} ms (Avg: {long_avg:.2} ms)"),
        _ => "Frame Time: n/a".to_string(),
    });

    let text = lines.join("\n");
    let painter = ctx.0.debug_painter();
    let font = egui::FontId::monospace(12.0);

    // Position in the lower-left corner with some padding
    let pos = [10.0, window.height() - 38.0 - font.size]; // 10px padding from bottom and left

    // Draw shadow for better contrast
    painter.text(
        egui::pos2(pos[0]+1.0, pos[1]+1.0),
        egui::Align2::LEFT_TOP,
        &text,
        font.clone(),
        shadow_color,
    );
    // Draw main text
    painter.text(
        egui::pos2(pos[0], pos[1]),
        egui::Align2::LEFT_TOP,
        &text,
        font,
        text_color,
    );
}

fn read_diagnostic(
    diagnostics: &DiagnosticsStore,
    path: bevy::diagnostic::DiagnosticPath,
) -> Option<f64> {
    diagnostics
        .get(&path)
        .and_then(|diagnostic| diagnostic.smoothed().or_else(|| diagnostic.value()))
}
