use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

mod overlay;

// Display frame timing and other per-frame data in the editor UI. This is a separate plugin to avoid adding the overhead of these systems when not in editor mode.
pub struct UIFrameDataPlugin;
impl Plugin for UIFrameDataPlugin {
    fn build(&self, app: &mut App) {
        // Add the overlay that displays stats on the screen (frame time)
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .init_resource::<overlay::FrameDataAverages>()
            .add_systems(
                Update,
                (
                    overlay::update_long_averages,
                    overlay::draw_framedata_overlay
                )
                    .chain()
            );
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
