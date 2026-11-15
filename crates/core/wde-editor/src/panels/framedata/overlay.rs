use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use crate::panels::framedata::read_diagnostic;

/// Frame rate and frame time averaged over a few seconds, shown on the right of the menu bar.
#[derive(Resource, Default)]
pub struct FrameDataAverages {
    pub(crate) fps: Option<f64>,
    pub(crate) frame_ms: Option<f64>
}

/// Controls the built-in frame data (FPS and frame time) drawn on the right of the menu bar. Consuming apps that
/// build their own status/footer UI showing the same data can set `enabled` to `false` here to avoid drawing both.
#[derive(Resource)]
pub struct FrameDataOverlayConfig {
    pub enabled: bool
}
impl Default for FrameDataOverlayConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
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
    let average = |prev: Option<f64>, value: f64| match prev {
        Some(prev) => prev + alpha * (value - prev),
        None => value
    };

    if let Some(fps) = read_diagnostic(&diagnostics, FrameTimeDiagnosticsPlugin::FPS) {
        averages.fps = Some(average(averages.fps, fps));
    }
    if let Some(frame_ms) = read_diagnostic(&diagnostics, FrameTimeDiagnosticsPlugin::FRAME_TIME) {
        averages.frame_ms = Some(average(averages.frame_ms, frame_ms));
    }
}
