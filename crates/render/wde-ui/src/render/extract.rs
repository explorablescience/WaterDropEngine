//! Builds the UI quads from the laid out nodes and uploads them to the GPU each frame.
use wde_logger::prelude::*;

use bevy::{prelude::*, window::PrimaryWindow};
use wde_renderer::prelude::{Color, *};

use crate::{
    node::{ComputedUiNode, UiBackground, UiBorderColor, UiCornerRadius},
    render::{
        data::{MAX_UI_QUADS, UiQuad, UiQuadData, UiQuadsHeader},
        text::UiTextDraw
    }
};

/// The UI quads to draw this frame, in draw order. Built in the main world and cloned to the render world.
#[derive(Resource, Clone, Default, ExtractResource)]
pub(crate) struct UiDrawList {
    pub viewport: Vec2,
    pub quads: Vec<UiQuad>,
    pub texts: Vec<UiTextDraw>
}

/// Number of quads to draw this frame, updated when the quads buffer is filled.
#[derive(Resource, Default)]
pub(crate) struct UiDrawCount(pub u32);

type UiQuadSource = (
    &'static ComputedUiNode,
    Option<&'static UiBackground>,
    Option<&'static UiBorderColor>,
    Option<&'static UiCornerRadius>
);

pub(crate) fn build_draw_list(
    window: Single<&Window, With<PrimaryWindow>>,
    nodes: Query<UiQuadSource>,
    mut draw_list: ResMut<UiDrawList>
) {
    let scale = window.scale_factor();
    let mut visible: Vec<_> = nodes
        .iter()
        .filter(|(node, background, border, _)| {
            (background.is_some() || border.is_some()) && node.size.cmpgt(Vec2::ZERO).all()
        })
        .collect();
    visible.sort_by_key(|(node, ..)| node.order);

    draw_list.viewport = window.physical_size().as_vec2();
    draw_list.quads.clear();
    draw_list.quads.extend(
        visible
            .into_iter()
            .map(|(node, background, border_color, radius)| {
                let position = node.position * scale;
                let size = node.size * scale;
                UiQuad {
                    rect: [position.x, position.y, size.x, size.y],
                    color: linear(background.map_or(Color::TRANSPARENT, |b| b.0)),
                    border_color: linear(border_color.map_or(Color::TRANSPARENT, |b| b.0)),
                    border: (node.border * scale).to_array(),
                    radius: [radius.map_or(0.0, |r| r.0) * scale, 0.0, 0.0, 0.0]
                }
            })
    );
}

fn linear(color: Color) -> [f32; 4] {
    let c = color.to_linear_rgba();
    [c.r(), c.g(), c.b(), c.a()]
}

pub(crate) fn update_ui_buffer(
    render_instance: Res<RenderInstance>,
    draw_list: Res<UiDrawList>,
    ui_data: ResRenderData<UiQuadData>,
    mut buffers: ResMut<RenderAssets<GpuBuffer>>,
    mut draw_count: ResMut<UiDrawCount>
) {
    let Some(buffer) = ui_data
        .iter()
        .next()
        .and_then(|(_, data)| data.get_buffer(UiQuadData::QUADS_IDX))
        .and_then(|buffer| buffers.get_mut(&buffer))
    else {
        return;
    };

    if draw_list.quads.len() > MAX_UI_QUADS {
        warn!(
            "Number of UI quads exceeded the maximum of {}. Some quads will be ignored.",
            MAX_UI_QUADS
        );
    }
    let quads = &draw_list.quads[..draw_list.quads.len().min(MAX_UI_QUADS)];
    draw_count.0 = quads.len() as u32;

    let header = UiQuadsHeader {
        viewport: [draw_list.viewport.x, draw_list.viewport.y, 0.0, 0.0]
    };
    let render_instance = render_instance.0.read().unwrap();
    buffer
        .buffer
        .write(&render_instance, bytemuck::bytes_of(&header), 0);
    if !quads.is_empty() {
        buffer.buffer.write(
            &render_instance,
            bytemuck::cast_slice(quads),
            std::mem::size_of::<UiQuadsHeader>()
        );
    }
}
