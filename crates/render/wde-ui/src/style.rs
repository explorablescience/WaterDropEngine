//! Layout style of UI nodes, following the CSS box model. Converted to taffy's style when laid out.
use taffy::{Dimension, LengthPercentage, LengthPercentageAuto};

pub use taffy::{
    AlignContent, AlignItems, AlignSelf, Display as UiDisplay, FlexDirection, FlexWrap,
    JustifyContent
};

/// How a node is positioned. There is no CSS `static`: every node is the containing block of
/// its absolute children, which keeps layout positions relative to the parent entity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiPosition {
    /// In the flow of its parent, offset by its inset.
    #[default]
    Relative,
    /// Out of the flow, placed by its inset relative to its parent.
    Absolute
}
impl From<UiPosition> for taffy::Position {
    fn from(position: UiPosition) -> Self {
        match position {
            UiPosition::Relative => taffy::Position::Relative,
            UiPosition::Absolute => taffy::Position::Absolute
        }
    }
}

/// A length.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Val {
    #[default]
    Auto,
    /// Logical pixels.
    Px(f32),
    /// Percentage (0 to 100) of the parent size.
    Percent(f32)
}
impl Val {
    fn dimension(self) -> Dimension {
        match self {
            Val::Auto => Dimension::auto(),
            Val::Px(v) => Dimension::length(v),
            Val::Percent(p) => Dimension::percent(p / 100.0)
        }
    }

    fn length_percentage_auto(self) -> LengthPercentageAuto {
        match self {
            Val::Auto => LengthPercentageAuto::auto(),
            Val::Px(v) => LengthPercentageAuto::length(v),
            Val::Percent(p) => LengthPercentageAuto::percent(p / 100.0)
        }
    }

    /// Padding, borders and gaps can't be auto: it resolves to zero.
    fn length_percentage(self) -> LengthPercentage {
        match self {
            Val::Auto => LengthPercentage::length(0.0),
            Val::Px(v) => LengthPercentage::length(v),
            Val::Percent(p) => LengthPercentage::percent(p / 100.0)
        }
    }
}

/// A length for each side of a node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiRect {
    pub left: Val,
    pub right: Val,
    pub top: Val,
    pub bottom: Val
}
impl UiRect {
    pub const ZERO: Self = Self::all(Val::Px(0.0));

    pub const fn all(val: Val) -> Self {
        Self::axes(val, val)
    }
    pub const fn axes(horizontal: Val, vertical: Val) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: vertical,
            bottom: vertical
        }
    }
    pub const fn px(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        Self {
            left: Val::Px(left),
            right: Val::Px(right),
            top: Val::Px(top),
            bottom: Val::Px(bottom)
        }
    }

    fn map<T>(self, f: impl Fn(Val) -> T) -> taffy::Rect<T> {
        taffy::Rect {
            left: f(self.left),
            right: f(self.right),
            top: f(self.top),
            bottom: f(self.bottom)
        }
    }
}
impl Default for UiRect {
    fn default() -> Self {
        Self::ZERO
    }
}

/// Layout style of a [`UiNode`](crate::node::UiNode). See the CSS properties of the same names.
#[derive(Clone, Debug, PartialEq)]
pub struct UiStyle {
    pub display: UiDisplay,
    pub position: UiPosition,
    /// Offsets of relative and absolute nodes.
    pub inset: UiRect,

    pub width: Val,
    pub height: Val,
    pub min_width: Val,
    pub min_height: Val,
    pub max_width: Val,
    pub max_height: Val,
    pub aspect_ratio: Option<f32>,

    pub margin: UiRect,
    pub padding: UiRect,
    /// Border widths, drawn with a [`UiBorderColor`](crate::node::UiBorderColor).
    pub border: UiRect,

    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Val,
    pub row_gap: Val,
    pub column_gap: Val,

    pub align_items: AlignItems,
    pub align_self: Option<AlignSelf>,
    pub align_content: AlignContent,
    pub justify_content: JustifyContent
}
impl Default for UiStyle {
    fn default() -> Self {
        let taffy: taffy::Style = taffy::Style::DEFAULT;
        Self {
            display: taffy.display,
            position: UiPosition::Relative,
            inset: UiRect::all(Val::Auto),
            width: Val::Auto,
            height: Val::Auto,
            min_width: Val::Auto,
            min_height: Val::Auto,
            max_width: Val::Auto,
            max_height: Val::Auto,
            aspect_ratio: None,
            margin: UiRect::ZERO,
            padding: UiRect::ZERO,
            border: UiRect::ZERO,
            flex_direction: taffy.flex_direction,
            flex_wrap: taffy.flex_wrap,
            flex_grow: taffy.flex_grow,
            flex_shrink: taffy.flex_shrink,
            flex_basis: Val::Auto,
            row_gap: Val::Px(0.0),
            column_gap: Val::Px(0.0),
            align_items: taffy.align_items,
            align_self: taffy.align_self,
            align_content: taffy.align_content,
            justify_content: taffy.justify_content
        }
    }
}
impl UiStyle {
    pub(crate) fn to_taffy(&self) -> taffy::Style {
        taffy::Style {
            display: self.display,
            position: self.position.into(),
            inset: self.inset.map(Val::length_percentage_auto),
            size: taffy::Size {
                width: self.width.dimension(),
                height: self.height.dimension()
            },
            min_size: taffy::Size {
                width: self.min_width.length_percentage_auto(),
                height: self.min_height.length_percentage_auto()
            },
            max_size: taffy::Size {
                width: self.max_width.length_percentage_auto(),
                height: self.max_height.length_percentage_auto()
            },
            aspect_ratio: self.aspect_ratio,
            margin: self.margin.map(Val::length_percentage_auto),
            padding: self.padding.map(Val::length_percentage),
            border: self.border.map(Val::length_percentage),
            flex_direction: self.flex_direction,
            flex_wrap: self.flex_wrap,
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            flex_basis: self.flex_basis.dimension(),
            gap: taffy::Size {
                width: self.column_gap.length_percentage(),
                height: self.row_gap.length_percentage()
            },
            align_items: self.align_items,
            align_self: self.align_self,
            align_content: self.align_content,
            justify_content: self.justify_content,
            ..taffy::Style::DEFAULT
        }
    }
}
