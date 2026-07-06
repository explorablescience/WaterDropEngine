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
    data::{GizmoLineData, GizmoLinesBinding},
    extract::{ExtractedGizmoLines, GizmoDrawCount, extract_gizmo_lines, update_gizmo_buffer},
    pipeline::GizmoRenderPipeline,
    subpass::SubRenderPassGizmos
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
}
impl Gizmos {
    pub fn line(&mut self, start: Vec3, end: Vec3, color: Color) {
        self.lines.push((start, end, color));
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
            Vec3::new(-half_size.x, half_size.y, half_size.z),
        ];

        let edges = [
            (0, 1), (1, 2), (2, 3), (3, 0), // back face
            (4, 5), (5, 6), (6, 7), (7, 4), // front face
            (0, 4), (1, 5), (2, 6), (3, 7), // sides
        ];

        for &(start_idx, end_idx) in &edges {
            let start = transform.transform_point(vertices[start_idx]);
            let end = transform.transform_point(vertices[end_idx]);
            self.line(start, end, color);
        }
    }

    /// Takes the recorded lines, leaving this resource empty for the next frame.
    fn take(&mut self) -> Vec<(Vec3, Vec3, Color)> {
        std::mem::take(&mut self.lines)
    }
}

pub struct GizmosPlugin;
impl Plugin for GizmosPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Gizmos>();

        app.add_plugins((
            RenderDataRegisterPlugin::<GizmoLineData>::default(),
            RenderBindingRegisterPlugin::<GizmoLinesBinding>::default(),
            RenderPipelineRegisterPlugin::<GizmoRenderPipeline>::default()
        ));

        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<ExtractedGizmoLines>()
            .init_resource::<GizmoDrawCount>()
            .add_systems(Extract, extract_gizmo_lines)
            .add_systems(Render, update_gizmo_buffer.in_set(RenderSet::Prepare));
    }

    fn finish(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap()
            .add_sub_pass::<SubRenderPassGizmos, RenderPassTransparent>();
    }
}
