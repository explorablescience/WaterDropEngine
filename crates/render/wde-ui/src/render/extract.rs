//! Builds the UI quads from the laid out nodes and uploads them to the GPU each frame.
use wde_logger::prelude::*;

use bevy::{prelude::*, window::PrimaryWindow};
use wde_renderer::prelude::{WdeColor, *};

use crate::{
    node::{ComputedUiNode, UiBackground, UiBorderColor, UiCornerRadius, UiImage},
    render::{
        data::{MAX_UI_QUADS, UiQuad, UiQuadData, UiQuadsHeader},
        image::{UiDrawBatches, UiImageBindGroups},
        text::UiTextDraw
    }
};

/// The UI quads to draw this frame, in draw order. Built in the main world and cloned to the render world.
#[derive(Resource, Clone, Default, ExtractResource)]
pub(crate) struct UiDrawList {
    pub viewport: Vec2,
    pub quads: Vec<UiQuad>,
    /// Image of each quad.
    pub images: Vec<Option<AssetId<Texture>>>,
    pub texts: Vec<UiTextDraw>
}

type UiQuadSource = (
    &'static ComputedUiNode,
    Option<&'static UiBackground>,
    Option<&'static UiBorderColor>,
    Option<&'static UiCornerRadius>,
    Option<&'static UiImage>
);

pub(crate) fn build_draw_list(
    window: Single<&Window, With<PrimaryWindow>>,
    nodes: Query<UiQuadSource>,
    mut draw_list: ResMut<UiDrawList>
) {
    let scale = window.scale_factor();
    let mut visible: Vec<_> = nodes
        .iter()
        .filter(|(node, background, border, _, image)| {
            (background.is_some() || border.is_some() || image.is_some())
                && node.size.cmpgt(Vec2::ZERO).all()
        })
        .collect();
    visible.sort_by_key(|(node, ..)| node.order);

    draw_list.viewport = window.physical_size().as_vec2();
    draw_list.quads.clear();
    draw_list.images.clear();
    for (node, background, border_color, radius, image) in visible {
        let position = node.position * scale;
        let size = node.size * scale;
        let color = match image {
            Some(image) => image.tint,
            None => background.map_or(WdeColor::TRANSPARENT, |b| b.0)
        };
        draw_list.quads.push(UiQuad {
            rect: [position.x, position.y, size.x, size.y],
            color: linear(color),
            border_color: linear(border_color.map_or(WdeColor::TRANSPARENT, |b| b.0)),
            border: (node.border * scale).to_array(),
            radius: [
                radius.map_or(0.0, |r| r.0) * scale,
                image.map_or(0.0, |_| 1.0),
                0.0,
                0.0
            ]
        });
        draw_list.images.push(image.map(|image| image.texture.id()));
    }
}

fn linear(color: WdeColor) -> [f32; 4] {
    let c = color.to_linear_rgba();
    [c.r(), c.g(), c.b(), c.a()]
}

/// Uploads the quads, and splits them into draw calls by image.
pub(crate) fn update_ui_buffer(
    render_instance: Res<RenderInstance>,
    draw_list: Res<UiDrawList>,
    ui_data: ResRenderData<UiQuadData>,
    mut buffers: ResMut<RenderAssets<GpuBuffer>>,
    (textures, placeholder): (
        Res<RenderAssets<GpuTexture>>,
        Option<Res<PlaceholderTexture>>
    ),
    mut bind_groups: ResMut<UiImageBindGroups>,
    mut batches: ResMut<UiDrawBatches>
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
    let count = draw_list.quads.len().min(MAX_UI_QUADS);
    let mut quads = draw_list.quads[..count].to_vec();
    let render_instance = render_instance.0.read().unwrap();
    // Textures were (re)loaded: their bind groups may point to dropped views
    if textures.is_changed() {
        bind_groups.clear();
    }
    batches.0 = bind_groups.batch(
        &render_instance,
        &textures,
        placeholder.map(|p| p.0.id()),
        &mut quads,
        &draw_list.images[..count]
    );

    let header = UiQuadsHeader {
        viewport: [draw_list.viewport.x, draw_list.viewport.y, 0.0, 0.0]
    };
    buffer
        .buffer
        .write(&render_instance, bytemuck::bytes_of(&header), 0);
    if !quads.is_empty() {
        buffer.buffer.write(
            &render_instance,
            bytemuck::cast_slice(&quads),
            std::mem::size_of::<UiQuadsHeader>()
        );
    }
}
