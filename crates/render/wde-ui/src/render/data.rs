//! GPU-side storage for the UI quads drawn each frame.
use bevy::{ecs::system::SystemParamItem, prelude::*};
use wde_renderer::prelude::*;

/// Maximum number of quads drawn in a single frame.
pub const MAX_UI_QUADS: usize = 16_384;

/// A UI node drawn as a rounded rectangle, in physical pixels. Colors are linear RGBA.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug, Default)]
pub(crate) struct UiQuad {
    /// Top-left corner (x, y) and size (z, w).
    pub rect: [f32; 4],
    pub color: [f32; 4],
    pub border_color: [f32; 4],
    /// Left, top, right, bottom.
    pub border: [f32; 4],
    /// Corner radius in x, 1 in y if the quad samples its image, the rest is padding.
    pub radius: [f32; 4]
}

/// Header of the quads buffer, followed by the [`UiQuad`]s.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug, Default)]
pub(crate) struct UiQuadsHeader {
    /// Viewport size in physical pixels in xy, the rest is padding.
    pub viewport: [f32; 4]
}

#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct UiQuadData;
impl UiQuadData {
    pub const QUADS_IDX: u32 = 0;
}
impl RenderData for UiQuadData {
    type Params = ();

    fn describe(_params: &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        builder.add_buffer(
            Self::QUADS_IDX,
            Buffer {
                label: "ui-quads-gpu".to_string(),
                size: std::mem::size_of::<UiQuadsHeader>()
                    + std::mem::size_of::<UiQuad>() * MAX_UI_QUADS,
                usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                content: None
            }
        );
    }
}

/// Render binding exposing the UI quads buffer as a read-only storage buffer.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct UiQuadsBinding;
impl RenderBinding for UiQuadsBinding {
    type Params = SRenderData<UiQuadData>;

    fn describe(
        &mut self,
        ui_quads: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder.add_buffer(ui_quads, UiQuadData::QUADS_IDX);
    }

    fn label(&self) -> &'static str {
        "ui_quads_binding"
    }
}
