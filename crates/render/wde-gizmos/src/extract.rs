//! Extracts recorded gizmo lines from the main world and uploads them to the GPU each frame.
use bevy::prelude::*;
use wde_logger::prelude::*;
use wde_renderer::prelude::{Color, *};

use crate::{
    Gizmos,
    data::{GizmoLineData, GizmoQuadData, GizmoVertex, MAX_GIZMO_LINES, MAX_GIZMO_QUADS}
};

/// Lines recorded on [`Gizmos`] this frame, extracted from the main world.
#[derive(Resource, Default)]
pub(crate) struct ExtractedGizmoLines(pub Vec<(Vec3, Vec3, Color)>);

/// Number of vertices to draw this frame, updated when the gizmo lines vertex buffer is filled.
#[derive(Resource, Default)]
pub(crate) struct GizmoDrawCount(pub u32);

/// Quads recorded on [`Gizmos`] this frame, extracted from the main world.
#[derive(Resource, Default)]
pub(crate) struct ExtractedGizmoQuads(pub Vec<([Vec3; 4], Color)>);

/// Number of vertices to draw this frame, updated when the gizmo quads vertex buffer is filled.
#[derive(Resource, Default)]
pub(crate) struct GizmoQuadDrawCount(pub u32);

/// Takes the lines and quads recorded on the [`Gizmos`] resource in the main world, clearing it
/// so the caller can record a fresh set of shapes for the next frame.
pub(crate) fn extract_gizmo_lines(
    mut main_world: ResMut<MainWorld>,
    mut extracted_lines: ResMut<ExtractedGizmoLines>,
    mut extracted_quads: ResMut<ExtractedGizmoQuads>
) {
    if let Some(mut gizmos) = main_world.get_resource_mut::<Gizmos>() {
        extracted_lines.0 = gizmos.take_lines();
        extracted_quads.0 = gizmos.take_quads();
    }
}

/// Uploads the extracted line vertices to the gizmo lines GPU buffer.
pub(crate) fn update_gizmo_buffer(
    render_instance: Res<RenderInstance>,
    extracted: Res<ExtractedGizmoLines>,
    gizmo_data: ResRenderData<GizmoLineData>,
    mut buffers: ResMut<RenderAssets<GpuBuffer>>,
    mut draw_count: ResMut<GizmoDrawCount>
) {
    let gizmo_buffer = match gizmo_data.iter().next() {
        Some((_, data)) => {
            match buffers.get_mut(&data.get_buffer(GizmoLineData::VERTICES_IDX).unwrap()) {
                Some(buffer) => buffer,
                None => return
            }
        }
        None => return
    };

    let lines = &extracted.0;
    if lines.len() > MAX_GIZMO_LINES {
        warn!(
            "Number of gizmo lines exceeded the maximum of {}. Some lines will be ignored.",
            MAX_GIZMO_LINES
        );
    }

    let mut vertices = Vec::with_capacity(lines.len().min(MAX_GIZMO_LINES) * 2);
    for (start, end, color) in lines.iter().take(MAX_GIZMO_LINES) {
        let linear = color.to_linear_rgba();
        let color = [linear.r(), linear.g(), linear.b(), linear.a()];
        vertices.push(GizmoVertex {
            position: [start.x, start.y, start.z, 1.0],
            color
        });
        vertices.push(GizmoVertex {
            position: [end.x, end.y, end.z, 1.0],
            color
        });
    }
    draw_count.0 = vertices.len() as u32;

    let render_instance = render_instance.0.read().unwrap();
    gizmo_buffer
        .buffer
        .write(&render_instance, bytemuck::cast_slice(&vertices), 0);
}

/// Uploads the extracted quad vertices to the gizmo quads GPU buffer.
pub(crate) fn update_gizmo_quad_buffer(
    render_instance: Res<RenderInstance>,
    extracted: Res<ExtractedGizmoQuads>,
    gizmo_data: ResRenderData<GizmoQuadData>,
    mut buffers: ResMut<RenderAssets<GpuBuffer>>,
    mut draw_count: ResMut<GizmoQuadDrawCount>
) {
    let gizmo_buffer = match gizmo_data.iter().next() {
        Some((_, data)) => {
            match buffers.get_mut(&data.get_buffer(GizmoQuadData::VERTICES_IDX).unwrap()) {
                Some(buffer) => buffer,
                None => return
            }
        }
        None => return
    };

    let quads = &extracted.0;
    if quads.len() > MAX_GIZMO_QUADS {
        warn!(
            "Number of gizmo quads exceeded the maximum of {}. Some quads will be ignored.",
            MAX_GIZMO_QUADS
        );
    }

    let mut vertices = Vec::with_capacity(quads.len().min(MAX_GIZMO_QUADS) * 6);
    for (corners, color) in quads.iter().take(MAX_GIZMO_QUADS) {
        let linear = color.to_linear_rgba();
        let color = [linear.r(), linear.g(), linear.b(), linear.a()];
        let to_vertex = |p: Vec3| GizmoVertex {
            position: [p.x, p.y, p.z, 1.0],
            color
        };

        // Two triangles covering the quad: (0, 1, 2) and (0, 2, 3).
        vertices.push(to_vertex(corners[0]));
        vertices.push(to_vertex(corners[1]));
        vertices.push(to_vertex(corners[2]));
        vertices.push(to_vertex(corners[0]));
        vertices.push(to_vertex(corners[2]));
        vertices.push(to_vertex(corners[3]));
    }
    draw_count.0 = vertices.len() as u32;

    let render_instance = render_instance.0.read().unwrap();
    gizmo_buffer
        .buffer
        .write(&render_instance, bytemuck::cast_slice(&vertices), 0);
}
