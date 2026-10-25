//! Rasterizes and draws the [`UiText`](crate::text::UiText) nodes with glyphon.
use std::sync::Arc;
use wde_logger::prelude::*;

use bevy::{ecs::system::SystemParamItem, prelude::*, window::PrimaryWindow};
use glyphon::{
    Buffer, Cache, Resolution, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport
};
use wde_renderer::prelude::{Color, *};

use crate::{
    layout::UiLayoutTree,
    node::ComputedUiNode,
    render::extract::UiDrawList,
    text::{UiFonts, UiText}
};

/// A text to draw, in physical pixels.
#[derive(Clone)]
pub(crate) struct UiTextDraw {
    pub buffer: Arc<Buffer>,
    /// Top-left corner of the text.
    pub position: Vec2,
    pub clip: Rect,
    pub color: Color,
    pub scale: f32
}

pub(crate) fn build_text_draws(
    window: Single<&Window, With<PrimaryWindow>>,
    tree: Res<UiLayoutTree>,
    texts: Query<(Entity, &ComputedUiNode, &UiText)>,
    mut draw_list: ResMut<UiDrawList>
) {
    let scale = window.scale_factor();
    draw_list.texts.clear();
    for (entity, node, text) in &texts {
        let Some(buffer) = tree.text_buffer(entity) else {
            continue;
        };
        if !node.size.cmpgt(Vec2::ZERO).all() {
            continue;
        }
        let content = node.position + node.border.xy() + node.padding.xy();
        draw_list.texts.push(UiTextDraw {
            buffer: buffer.clone(),
            position: content * scale,
            clip: Rect::from_corners(node.position * scale, (node.position + node.size) * scale),
            color: text.color,
            scale
        });
    }
}

#[derive(Resource)]
pub(crate) struct UiTextRenderer {
    atlas: TextAtlas,
    renderer: TextRenderer,
    viewport: Viewport,
    swash_cache: SwashCache
}
impl UiTextRenderer {
    pub(crate) fn new(world: &World) -> Self {
        let render_instance = world.resource::<RenderInstance>().0.read().unwrap();
        let (device, queue) = (&render_instance.device, &render_instance.queue);
        let cache = Cache::new(device);
        let mut atlas = TextAtlas::new(device, queue, &cache, SWAPCHAIN_FORMAT);
        let renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);
        Self {
            viewport: Viewport::new(device, &cache),
            atlas,
            renderer,
            swash_cache: SwashCache::new()
        }
    }
}

/// Uploads the glyphs of this frame's texts to the atlas, and their quads to the GPU.
pub(crate) fn prepare_text(
    render_instance: Res<RenderInstance>,
    draw_list: Res<UiDrawList>,
    fonts: Res<UiFonts>,
    text_renderer: Option<ResMut<UiTextRenderer>>
) {
    let Some(mut text_renderer) = text_renderer else {
        return;
    };
    let text_renderer = &mut *text_renderer;
    let render_instance = render_instance.0.read().unwrap();
    text_renderer.atlas.trim();
    text_renderer.viewport.update(
        &render_instance.queue,
        Resolution {
            width: draw_list.viewport.x as u32,
            height: draw_list.viewport.y as u32
        }
    );

    let areas = draw_list.texts.iter().map(|text| {
        let color = text.color.to_srgba();
        let channel = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        TextArea {
            buffer: &text.buffer,
            left: text.position.x,
            top: text.position.y,
            scale: text.scale,
            bounds: TextBounds {
                left: text.clip.min.x.floor() as i32,
                top: text.clip.min.y.floor() as i32,
                right: text.clip.max.x.ceil() as i32,
                bottom: text.clip.max.y.ceil() as i32
            },
            default_color: glyphon::Color::rgba(
                channel(color.r()),
                channel(color.g()),
                channel(color.b()),
                channel(color.a())
            ),
            custom_glyphs: &[]
        }
    });
    if let Err(e) = text_renderer.renderer.prepare(
        &render_instance.device,
        &render_instance.queue,
        &mut fonts.0.lock().unwrap(),
        &mut text_renderer.atlas,
        &text_renderer.viewport,
        areas,
        &mut text_renderer.swash_cache
    ) {
        error!("Failed to prepare the UI text: {}.", e);
    }
}

/// Draws the texts after all the quads: a node overlapping a text does not hide it.
pub(crate) struct SubRenderPassUiText;
impl RenderSubPass for SubRenderPassUiText {
    type Params = ();

    fn describe(_params: &SystemParamItem<Self::Params>) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![SubPassCommand::Custom(draw_text)])
    }

    fn label() -> &'static str {
        "ui-text"
    }
}

fn draw_text<'pass>(world: &'pass World, render_pass: &mut RenderPassInstance<'pass>) {
    let Some(text_renderer) = world.get_resource::<UiTextRenderer>() else {
        return;
    };
    if let Err(e) = text_renderer.renderer.render(
        &text_renderer.atlas,
        &text_renderer.viewport,
        render_pass.raw_mut()
    ) {
        error!("Failed to draw the UI text: {}.", e);
    }
}
