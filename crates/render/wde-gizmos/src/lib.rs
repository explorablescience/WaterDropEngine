//! WaterDropEngine's `wde-gizmos` crate renders lightweight debug gizmos (lines/boxes) on top of the main scene.
//!
//! Lines are recorded on the [`Gizmos`] resource from any main-world system (e.g. `ResMut<Gizmos>`).
//! Every frame, the recorded lines are extracted, uploaded to the GPU and drawn in a single
//! line-list draw call in the [`RenderPassTransparent`](wde_pbr::prelude::RenderPassTransparent)
//! pass, then the [`Gizmos`] resource is cleared so it can be recorded again for the next frame.
use bevy::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::{Color, *};

use crate::{
    data::{GizmoLineData, GizmoLinesBinding, GizmoQuadData, GizmoQuadsBinding},
    extract::{
        ExtractedGizmoLines, ExtractedGizmoQuads, GizmoDrawCount, GizmoQuadDrawCount,
        extract_gizmo_lines, update_gizmo_buffer, update_gizmo_quad_buffer
    },
    pipeline::{GizmoQuadRenderPipeline, GizmoRenderPipeline},
    subpass::{SubRenderPassGizmoQuads, SubRenderPassGizmos}
};

mod data;
mod extract;
mod pipeline;
mod subpass;

#[doc(hidden)]
pub mod prelude {
    pub use crate::Gizmos;
}

#[derive(Resource, Default)]
pub struct Gizmos {
    lines: Vec<(Vec3, Vec3, Color)>,
    quads: Vec<([Vec3; 4], Color)>
}
impl Gizmos {
    pub fn line(&mut self, start: Vec3, end: Vec3, color: Color) {
        self.lines.push((start, end, color));
    }

    /// Records a filled quad from 4 world-space corners, in order (either winding), with the
    /// given color. Use a color with alpha < 1.0 for a translucent quad.
    pub fn quad(&mut self, corners: [Vec3; 4], color: Color) {
        self.quads.push((corners, color));
    }

    pub fn cube(&mut self, transform: Transform, color: Color) {
        let half_size = Vec3::splat(0.5);
        let vertices = [
            Vec3::new(-half_size.x, -half_size.y, -half_size.z),
            Vec3::new(half_size.x, -half_size.y, -half_size.z),
            Vec3::new(half_size.x, half_size.y, -half_size.z),
            Vec3::new(-half_size.x, half_size.y, -half_size.z),
            Vec3::new(-half_size.x, -half_size.y, half_size.z),
            Vec3::new(half_size.x, -half_size.y, half_size.z),
            Vec3::new(half_size.x, half_size.y, half_size.z),
            Vec3::new(-half_size.x, half_size.y, half_size.z)
        ];

        let edges = [
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0), // back face
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 4), // front face
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7) // sides
        ];

        for &(start_idx, end_idx) in &edges {
            let start = transform.transform_point(vertices[start_idx]);
            let end = transform.transform_point(vertices[end_idx]);
            self.line(start, end, color);
        }
    }

    /// Takes the recorded lines, leaving this resource empty for the next frame.
    fn take_lines(&mut self) -> Vec<(Vec3, Vec3, Color)> {
        std::mem::take(&mut self.lines)
    }

    /// Takes the recorded quads, leaving this resource empty for the next frame.
    fn take_quads(&mut self) -> Vec<([Vec3; 4], Color)> {
        std::mem::take(&mut self.quads)
    }
}

pub struct GizmosPlugin;
impl Plugin for GizmosPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Gizmos>();

        app.add_plugins((
            RenderDataRegisterPlugin::<GizmoLineData>::default(),
            RenderBindingRegisterPlugin::<GizmoLinesBinding>::default(),
            RenderPipelineRegisterPlugin::<GizmoRenderPipeline>::default(),
            RenderDataRegisterPlugin::<GizmoQuadData>::default(),
            RenderBindingRegisterPlugin::<GizmoQuadsBinding>::default(),
            RenderPipelineRegisterPlugin::<GizmoQuadRenderPipeline>::default()
        ));

        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<ExtractedGizmoLines>()
            .init_resource::<ExtractedGizmoQuads>()
            .init_resource::<GizmoDrawCount>()
            .init_resource::<GizmoQuadDrawCount>()
            .add_systems(Extract, extract_gizmo_lines)
            .add_systems(
                Render,
                (update_gizmo_quad_buffer, update_gizmo_buffer).in_set(RenderSet::Prepare)
            );
    }

    fn finish(&self, app: &mut App) {
        // Quads are added first so lines are drawn on top of quad fills.
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap()
            .add_sub_pass::<SubRenderPassGizmoQuads, RenderPassTransparent>()
            .add_sub_pass::<SubRenderPassGizmos, RenderPassTransparent>();
    }
}
