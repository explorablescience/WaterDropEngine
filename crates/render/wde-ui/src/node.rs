//! Components describing UI nodes: their layout style and how they are drawn.
use bevy::prelude::*;
use wde_renderer::prelude::Color;

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

/// Fill color of a [`UiNode`]. Nodes without one are invisible (and let the pointer through).
#[derive(Component, Clone, Copy, Debug)]
pub struct UiBackground(pub Color);

/// Color of the border, whose widths are set by [`UiStyle::border`].
#[derive(Component, Clone, Copy, Debug)]
pub struct UiBorderColor(pub Color);

/// Radius of the node corners, in logical pixels.
#[derive(Component, Clone, Copy, Default, Debug)]
pub struct UiCornerRadius(pub f32);
