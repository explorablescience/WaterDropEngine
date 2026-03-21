use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use wde_egui::prelude::*;

use crate::panels::framedata::read_diagnostic;


#[derive(Resource, Default)]
pub struct FrameDataAverages {
    fps_long_avg: Option<f64>,
    frame_ms_long_avg: Option<f64>,
}

pub fn update_long_averages(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    mut averages: ResMut<FrameDataAverages>,
) {
    let frame_count = read_diagnostic(&diagnostics, FrameTimeDiagnosticsPlugin::FRAME_COUNT);
    if frame_count.is_some_and(|count| count < 10.0) {
        // Not enough data yet to compute stable averages
        return;
    }

    // Exponential moving average with a long time constant for stable debug trends.
    let period_average_seconds: f64 = 3.0;
    let alpha = (time.delta_secs_f64() / period_average_seconds).clamp(0.0, 1.0);

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

pub fn draw_framedata_overlay(
    ctx: Res<EguiContext>,
    diagnostics: Res<DiagnosticsStore>,
    averages: Res<FrameDataAverages>,
    window: Query<&Window, With<PrimaryWindow>>
) {
    let Some(window) = window.single().ok() else {
        return; // No primary window, can't draw overlay
    };
    let frame_count = read_diagnostic(&diagnostics, FrameTimeDiagnosticsPlugin::FRAME_COUNT);

    let text_color = egui::Color32::from_gray(140);
    let mut lines = Vec::with_capacity(3);
    lines.push(match averages.fps_long_avg {
        Some(long_avg) => format!("FPS: {long_avg:.1}"),
        _ => "FPS: n/a".to_string(),
    });
    lines.push(match averages.frame_ms_long_avg {
        Some(long_avg) => format!("Frame Time: {long_avg:.2} ms"),
        _ => "Frame Time: n/a".to_string(),
    });
    lines.push(match frame_count {
        Some(value) => format!("Frame Count: {:.0}", value),
        None => "Frame Count: n/a".to_string(),
    });

    let text = lines.join("\n");
    let painter = ctx.0.debug_painter();
    let font = egui::FontId::monospace(12.0);

    // Position in the lower-left corner with some padding
    let pos = [10.0, window.height() - 38.0 - font.size]; // 10px padding from bottom and left

    // Draw main text
    painter.text(
        egui::pos2(pos[0], pos[1]),
        egui::Align2::LEFT_TOP,
        &text,
        font,
        text_color,
    );
}
