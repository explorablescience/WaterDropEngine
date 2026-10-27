//! Text nodes: shaped with cosmic-text for layout, rasterized and drawn with glyphon.
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use glyphon::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Weight};
use wde_renderer::prelude::WdeColor;

use crate::node::UiNode;

/// Line height, relative to the font size.
const LINE_HEIGHT: f32 = 1.25;

/// A text leaf node. Its size is measured from the text, wrapped to the width given by its parent.
#[derive(Component, Clone, Debug, PartialEq)]
#[require(UiNode)]
pub struct UiText {
    pub text: String,
    /// In logical pixels.
    pub font_size: f32,
    pub color: WdeColor,
    pub bold: bool
}
impl UiText {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font_size: 16.0,
            color: WdeColor::WHITE,
            bold: false
        }
    }
    pub fn with_size(self, font_size: f32) -> Self {
        Self { font_size, ..self }
    }
    pub fn with_color(self, color: WdeColor) -> Self {
        Self { color, ..self }
    }
    pub fn bold(self) -> Self {
        Self { bold: true, ..self }
    }
}

/// Font database and shaping caches, shared by the main world (layout) and the render world (rasterization).
#[derive(Resource, Clone)]
pub(crate) struct UiFonts(pub Arc<Mutex<FontSystem>>);
impl Default for UiFonts {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(FontSystem::new())))
    }
}

/// Shaped text of a [`UiText`] node, held as its taffy node context. Shared with the render
/// world, so it is only cloned when reshaped while the render world still draws it.
pub(crate) struct UiTextBuffer(pub Arc<Buffer>);

pub(crate) fn shape(font_system: &mut FontSystem, text: &UiText) -> UiTextBuffer {
    let mut buffer = Buffer::new(
        font_system,
        Metrics::new(text.font_size, text.font_size * LINE_HEIGHT)
    );
    let weight = if text.bold {
        Weight::BOLD
    } else {
        Weight::NORMAL
    };
    buffer.set_text(
        font_system,
        &text.text,
        &Attrs::new().family(Family::SansSerif).weight(weight),
        Shaping::Advanced
    );
    UiTextBuffer(Arc::new(buffer))
}

/// Wraps the text to `width` (unbounded if `None`), and returns its size.
pub(crate) fn measure(
    font_system: &mut FontSystem,
    buffer: &mut Arc<Buffer>,
    width: Option<f32>
) -> Vec2 {
    // Reshapes on resize only
    if buffer.size().0 != width {
        Arc::make_mut(buffer).set_size(font_system, width, None);
    }
    let (width, lines) = buffer
        .layout_runs()
        .fold((0.0f32, 0), |(width, lines), run| {
            (width.max(run.line_w), lines + 1)
        });
    Vec2::new(width, lines as f32 * buffer.metrics().line_height)
}
