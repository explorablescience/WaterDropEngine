use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

mod overlay;

pub use overlay::{FrameDataAverages, FrameDataOverlayConfig};

/// Averages the frame timings shown in the menu bar.
pub struct UIFrameDataPlugin;
impl Plugin for UIFrameDataPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .init_resource::<overlay::FrameDataAverages>()
            .init_resource::<FrameDataOverlayConfig>()
            .add_systems(Update, overlay::update_long_averages);
    }
}

/// Helper function to read a diagnostic value, preferring the smoothed value if available for more stable readings in the UI.
/// Returns None if the diagnostic or value is not available.
pub(crate) fn read_diagnostic(
    diagnostics: &DiagnosticsStore,
    path: bevy::diagnostic::DiagnosticPath
) -> Option<f64> {
    diagnostics
        .get(&path)
        .and_then(|diagnostic| diagnostic.smoothed().or_else(|| diagnostic.value()))
}
