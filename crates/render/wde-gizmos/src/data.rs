//! GPU-side storage for the gizmo line vertices recorded each frame.
use bevy::{ecs::system::SystemParamItem, prelude::*};
use wde_renderer::prelude::*;

/// Maximum number of lines that can be recorded in a single frame. Lines recorded past this
/// limit are dropped (with a warning) rather than growing the GPU buffer.
pub const MAX_GIZMO_LINES: usize = 32_768 * 2;

/// A single gizmo line vertex: world-space position and linear RGBA color.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug, Default)]
pub(crate) struct GizmoVertex {
    pub position: [f32; 4],
    pub color: [f32; 4]
}

/// GPU storage buffer holding the vertices of the gizmo lines recorded this frame (2 vertices
/// per line, vertex-pulled by index in the gizmo vertex shader).
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct GizmoLineData;
impl GizmoLineData {
    pub const VERTICES_IDX: u32 = 0;
}
impl RenderData for GizmoLineData {
    type Params = ();

    fn describe(_params: &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        builder.add_buffer(
            Self::VERTICES_IDX,
            Buffer {
                label: "gizmo-lines-vertices-gpu".to_string(),
                size: std::mem::size_of::<GizmoVertex>() * MAX_GIZMO_LINES * 2,
                usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                content: None
            }
        );
    }
}

/// Render binding exposing the gizmo lines vertex buffer as a read-only storage buffer.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct GizmoLinesBinding;
impl RenderBinding for GizmoLinesBinding {
    type Params = SRenderData<GizmoLineData>;

    fn describe(
        &mut self,
        gizmo_lines: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder.add_buffer(gizmo_lines, GizmoLineData::VERTICES_IDX);
    }

    fn label(&self) -> &'static str {
        "gizmo_lines_binding"
    }
}

/// Maximum number of quads that can be recorded in a single frame. Quads recorded past this
/// limit are dropped (with a warning) rather than growing the GPU buffer.
pub const MAX_GIZMO_QUADS: usize = 16_384;

/// GPU storage buffer holding the vertices of the gizmo quads recorded this frame (2 triangles,
/// i.e. 6 vertices, per quad, vertex-pulled by index in the gizmo vertex shader).
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct GizmoQuadData;
impl GizmoQuadData {
    pub const VERTICES_IDX: u32 = 0;
}
impl RenderData for GizmoQuadData {
    type Params = ();

    fn describe(_params: &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        builder.add_buffer(
            Self::VERTICES_IDX,
            Buffer {
                label: "gizmo-quads-vertices-gpu".to_string(),
                size: std::mem::size_of::<GizmoVertex>() * MAX_GIZMO_QUADS * 6,
                usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                content: None
            }
        );
    }
}

/// Render binding exposing the gizmo quads vertex buffer as a read-only storage buffer.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct GizmoQuadsBinding;
impl RenderBinding for GizmoQuadsBinding {
    type Params = SRenderData<GizmoQuadData>;

    fn describe(
        &mut self,
        gizmo_quads: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder.add_buffer(gizmo_quads, GizmoQuadData::VERTICES_IDX);
    }

    fn label(&self) -> &'static str {
        "gizmo_quads_binding"
    }
}
