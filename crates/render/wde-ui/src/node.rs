//! Components describing UI nodes: their layout style and how they are drawn.
use bevy::prelude::*;
use wde_renderer::prelude::{WdeColor, Texture};

use crate::style::UiStyle;

/// A UI node, laid out by taffy from its [`UiStyle`]. Its UI children are the entity's
/// [`Children`] that are also [`UiNode`]s. A node without a [`UiNode`] parent is a root, laid out
/// against the whole window.
#[derive(Component, Clone, Default, Debug)]
#[require(ComputedUiNode)]
pub struct UiNode(pub UiStyle);

/// Layout result of a [`UiNode`], in logical window pixels (origin at the top-left corner).
#[derive(Component, Clone, Copy, Default, Debug, PartialEq)]
pub struct ComputedUiNode {
    /// Top-left corner of the node.
    pub position: Vec2,
    pub size: Vec2,
    /// Border widths: left, top, right, bottom.
    pub border: Vec4,
    /// Padding: left, top, right, bottom.
    pub padding: Vec4,
    /// Draw order: nodes with a higher order are drawn on top.
    pub order: u32
}
impl ComputedUiNode {
    pub fn rect(&self) -> Rect {
        Rect::from_corners(self.position, self.position + self.size)
    }
}

/// Fill color of a [`UiNode`]. Nodes without one (nor a [`UiImage`]) are invisible, and let the
/// pointer through.
#[derive(Component, Clone, Copy, Debug)]
pub struct UiBackground(pub WdeColor);

/// Color of the border, whose widths are set by [`UiStyle::border`].
#[derive(Component, Clone, Copy, Debug)]
pub struct UiBorderColor(pub WdeColor);

/// Radius of the node corners, in logical pixels.
#[derive(Component, Clone, Copy, Default, Debug)]
pub struct UiCornerRadius(pub f32);

/// Image stretched over the whole node, in place of its [`UiBackground`]. The texture holds sRGB
/// data, as loaded by default.
#[derive(Component, Clone, Debug)]
pub struct UiImage {
    pub texture: Handle<Texture>,
    /// Multiplies the image colors.
    pub tint: WdeColor
}
impl UiImage {
    pub fn new(texture: Handle<Texture>) -> Self {
        Self {
            texture,
            tint: WdeColor::WHITE
        }
    }
    pub fn with_tint(self, tint: WdeColor) -> Self {
        Self { tint, ..self }
    }
}
