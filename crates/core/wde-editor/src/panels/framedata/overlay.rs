use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use wde_egui::prelude::*;

use crate::panels::framedata::read_diagnostic;
use crate::ui::UIMenu;

#[derive(Resource, Default)]
pub struct FrameDataAverages {
    fps_long_avg: Option<f64>,
    frame_ms_long_avg: Option<f64>
}

pub fn update_long_averages(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    mut averages: ResMut<FrameDataAverages>
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
            None => fps
        });
    }

    if let Some(frame_ms) = read_diagnostic(&diagnostics, FrameTimeDiagnosticsPlugin::FRAME_TIME) {
        averages.frame_ms_long_avg = Some(match averages.frame_ms_long_avg {
            Some(prev) => prev + alpha * (frame_ms - prev),
            None => frame_ms
        });
    }
}

pub fn draw_framedata_overlay(
    ctx: Res<EguiContext>,
    diagnostics: Res<DiagnosticsStore>,
    averages: Res<FrameDataAverages>,
    window: Query<&Window, With<PrimaryWindow>>,
    ui_menu: Res<UIMenu>
) {
    let Some(window) = window.single().ok() else {
        return; // No primary window, can't draw overlay
    };
    let frame_count = read_diagnostic(&diagnostics, FrameTimeDiagnosticsPlugin::FRAME_COUNT);

    let text_color = egui::Color32::from_gray(140);
    let mut lines = Vec::with_capacity(3);
    lines.push(match averages.fps_long_avg {
        Some(long_avg) => format!("FPS: {long_avg:.1}"),
        _ => "FPS: n/a".to_string()
    });
    lines.push(match averages.frame_ms_long_avg {
        Some(long_avg) => format!("Frame Time: {long_avg:.2} ms"),
        _ => "Frame Time: n/a".to_string()
    });
    lines.push(match frame_count {
        Some(value) => format!("Frame Count: {:.0}", value),
        None => "Frame Count: n/a".to_string()
    });

    let text = lines.join("\n");
    let painter = ctx.0.debug_painter();
    let font = egui::FontId::monospace(12.0);
    let galley = painter.layout_no_wrap(text, font, text_color);

    // Position in the lower-left corner with some padding, backed by a rounded card so the
    // text stays legible over any background (viewport, grid, etc).
    let margin = 10.0;
    let padding = egui::vec2(8.0, 6.0);
    let card_size = galley.size() + padding * 2.0;
    let card_rect = egui::Rect::from_min_size(
        egui::pos2(margin, window.height() - margin - card_size.y),
        card_size
    );

    let bg_color = ui_menu.style().map_or(egui::Color32::from_gray(0), |style| style.visuals.extreme_bg_color);
    painter.rect_filled(card_rect, 6.0, bg_color);
    painter.galley(card_rect.min + padding, galley, text_color);
}
