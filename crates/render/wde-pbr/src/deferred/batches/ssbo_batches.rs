use bevy::prelude::*;
use wde_renderer::prelude::*;

/// The maximum number of batches in the ssbo.
pub const SSBO_MAX_BATCHES: usize = 100_000;

pub(crate) struct SsboBatchesPlugin;
impl Plugin for SsboBatchesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RenderBindingPluginRegister::<PbrSsboBatches>::default(),
            RenderBindingPluginRegister::<PbrSsboBatchesStaging>::default()
        ));
    }
}

/// Contains the transform data for the entities in the scene.
/// See [crate::deferred] level documentation for more details on how this is used, and how it is updated.
#[derive(Asset, Clone, TypePath, Default)]
pub struct PbrSsboBatches;
impl RenderBinding for PbrSsboBatches {
    fn describe(&self, builder: &mut RenderBindingBuilder) {
        builder.add_buffer(
            0,
            Buffer {
                label: "pbr-instance-to-transform-buffer-gpu".to_string(),
                size: std::mem::size_of::<u32>() * SSBO_MAX_BATCHES,
                usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                content: None
            }
        );
    }

    fn label(&self) -> &'static str {
        "pbr-ssbo-batches"
    }
}

#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct PbrSsboBatchesStaging;
impl RenderBinding for PbrSsboBatchesStaging {
    fn describe(&self, builder: &mut RenderBindingBuilder) {
        builder.add_buffer(
            0,
            Buffer {
                label: "pbr-instance-to-transform-staging".to_string(),
                size: std::mem::size_of::<u32>() * SSBO_MAX_BATCHES,
                usage: BufferUsage::COPY_SRC | BufferUsage::COPY_DST,
                content: None
            }
        );

        // As this is a pure staging cpu side buffer, don't need a bind group
        builder.no_bind_group();
    }

    fn label(&self) -> &'static str {
        "pbr-ssbo-batches-staging"
    }
}
