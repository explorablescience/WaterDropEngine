//! The editor's egui theme: a flat dark palette with a single accent color, the Inter font and compact spacing so
//! debug windows leave most of the screen to the game.
//!
//! Installed on the egui context by the [`EditorPlugin`](crate::EditorPlugin). Panels can use the [`palette`] to
//! stay consistent with it, and an app can still replace it with `ctx.set_style` / `ctx.set_fonts` at startup.
use std::sync::Arc;

use wde_egui::prelude::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Margin, Shadow, Stroke, TextStyle,
    style::WidgetVisuals
};

pub mod palette {
    use wde_egui::prelude::egui::Color32;

    // Backgrounds, darkest to lightest
    pub const BG_EXTREME: Color32 = Color32::from_rgb(14, 14, 15);
    pub const BG_PANEL: Color32 = Color32::from_rgb(23, 23, 24);
    pub const BG_CARD: Color32 = Color32::from_rgb(30, 30, 31);
    pub const BG_WIDGET: Color32 = Color32::from_rgb(37, 37, 39);
    pub const BG_WIDGET_HOVERED: Color32 = Color32::from_rgb(50, 50, 53);
    pub const BG_WIDGET_ACTIVE: Color32 = Color32::from_rgb(66, 66, 70);
    /// Slider rails and checkbox boxes, kept lighter than [`BG_WIDGET`] so they stand out from the window.
    pub const BG_RAIL: Color32 = Color32::from_rgb(52, 52, 55);

    // Borders and separators
    pub const SEPARATOR: Color32 = Color32::from_rgb(38, 38, 40);
    pub const BORDER: Color32 = Color32::from_rgb(48, 48, 51);
    pub const BORDER_STRONG: Color32 = Color32::from_rgb(72, 72, 76);

    // Text
    pub const TEXT: Color32 = Color32::from_rgb(240, 240, 240);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(200, 200, 202);
    pub const TEXT_WEAK: Color32 = Color32::from_rgb(130, 130, 134);
    pub const TEXT_DISABLED: Color32 = Color32::from_rgb(96, 96, 100);

    // The one interactive color, used sparingly: menu title, open windows, selection, hyperlinks.
    pub const ACCENT: Color32 = Color32::from_rgb(128, 186, 255);
    pub const ACCENT_HOVERED: Color32 = Color32::from_rgb(168, 210, 255);
    pub const ACCENT_MUTED: Color32 = Color32::from_rgb(70, 95, 135);
    pub const ACCENT_BG: Color32 = Color32::from_rgb(58, 88, 128);

    // Semantic, muted to match the accent
    pub const ERROR: Color32 = Color32::from_rgb(214, 112, 112);
    pub const WARNING: Color32 = Color32::from_rgb(219, 179, 92);
    pub const SUCCESS: Color32 = Color32::from_rgb(132, 188, 118);
    pub const INFO: Color32 = Color32::from_rgb(104, 189, 178);
}

pub mod layout {
    /// Windows and menus.
    pub const WINDOW_ROUNDING: u8 = 6;
    /// Buttons, sliders, text fields.
    pub const WIDGET_ROUNDING: u8 = 3;
}

pub mod fonts {
    pub const FONT_REGULAR: &str = "Inter";
    pub const FONT_SEMIBOLD: &str = "Inter-SemiBold";

    pub const SIZE_SMALL: f32 = 10.5;
    pub const SIZE_BODY: f32 = 12.0;
    pub const SIZE_HEADING: f32 = 12.5;
    pub const SIZE_MONOSPACE: f32 = 11.5;
}

use palette::*;

pub fn body_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn heading_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(fonts::FONT_SEMIBOLD.into()))
}

/// Installs the fonts and style on the context.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(font_definitions());
    ctx.set_style(style());
}

fn font_definitions() -> egui::FontDefinitions {
    let mut defs = egui::FontDefinitions::default();
    defs.font_data.insert(
        fonts::FONT_REGULAR.to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../fonts/Inter-Light.otf"
        )))
    );
    defs.font_data.insert(
        fonts::FONT_SEMIBOLD.to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../fonts/Inter-SemiBold.otf"
        )))
    );

    // egui's own fonts stay as fallbacks for symbols and emojis
    let proportional = defs.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, fonts::FONT_REGULAR.to_owned());
    let mut semibold = proportional.clone();
    semibold.insert(0, fonts::FONT_SEMIBOLD.to_owned());
    defs.families
        .insert(FontFamily::Name(fonts::FONT_SEMIBOLD.into()), semibold);
    defs
}

pub fn style() -> egui::Style {
    let mut style = egui::Style {
        text_styles: [
            (TextStyle::Small, body_font(fonts::SIZE_SMALL)),
            (TextStyle::Body, body_font(fonts::SIZE_BODY)),
            (TextStyle::Button, body_font(fonts::SIZE_BODY)),
            (TextStyle::Heading, heading_font(fonts::SIZE_HEADING)),
            (
                TextStyle::Monospace,
                FontId::new(fonts::SIZE_MONOSPACE, FontFamily::Monospace)
            )
        ]
        .into(),
        ..Default::default()
    };

    let spacing = &mut style.spacing;
    spacing.item_spacing = egui::vec2(8.0, 5.0);
    spacing.window_margin = Margin::symmetric(10, 8);
    spacing.menu_margin = Margin::same(5);
    spacing.button_padding = egui::vec2(8.0, 3.0);
    spacing.indent = 16.0;
    spacing.interact_size = egui::vec2(36.0, 20.0);
    spacing.slider_width = 130.0;
    spacing.slider_rail_height = 4.0;
    spacing.combo_width = 120.0;
    spacing.text_edit_width = 160.0;
    spacing.icon_width = 13.0;
    spacing.icon_width_inner = 7.0;
    spacing.icon_spacing = 5.0;
    spacing.menu_spacing = 2.0;
    spacing.scroll = egui::style::ScrollStyle {
        bar_width: 6.0,
        floating_allocated_width: 2.0,
        ..egui::style::ScrollStyle::floating()
    };

    let visuals = &mut style.visuals;
    visuals.dark_mode = true;
    visuals.override_text_color = None;
    visuals.weak_text_color = Some(TEXT_WEAK);
    visuals.hyperlink_color = ACCENT;
    visuals.warn_fg_color = WARNING;
    visuals.error_fg_color = ERROR;
    visuals.extreme_bg_color = BG_EXTREME;
    visuals.text_edit_bg_color = Some(BG_EXTREME);
    visuals.faint_bg_color = BG_CARD;
    visuals.code_bg_color = BG_EXTREME;
    visuals.panel_fill = BG_PANEL;
    visuals.selection.bg_fill = ACCENT_BG;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT_HOVERED);

    visuals.window_fill = BG_PANEL;
    visuals.window_stroke = Stroke::new(1.0, BORDER);
    visuals.window_corner_radius = CornerRadius::same(layout::WINDOW_ROUNDING);
    visuals.window_shadow = Shadow {
        offset: [0, 4],
        blur: 14,
        spread: 0,
        color: Color32::from_black_alpha(90)
    };
    visuals.menu_corner_radius = CornerRadius::same(layout::WINDOW_ROUNDING);
    visuals.popup_shadow = Shadow {
        offset: [0, 3],
        blur: 10,
        spread: 0,
        color: Color32::from_black_alpha(110)
    };
    visuals.resize_corner_size = 8.0;
    visuals.collapsing_header_frame = false;
    visuals.indent_has_left_vline = true;
    visuals.striped = false;
    visuals.slider_trailing_fill = true;
    visuals.handle_shape = egui::style::HandleShape::Rect { aspect_ratio: 0.5 };

    let base = WidgetVisuals {
        bg_fill: BG_WIDGET,
        weak_bg_fill: BG_WIDGET,
        bg_stroke: Stroke::NONE,
        fg_stroke: Stroke::new(1.0, TEXT_MUTED),
        corner_radius: CornerRadius::same(layout::WIDGET_ROUNDING),
        expansion: 0.0
    };
    let widgets = &mut visuals.widgets;
    widgets.noninteractive = WidgetVisuals {
        bg_fill: BG_CARD,
        weak_bg_fill: BG_CARD,
        // Separators, group frames and panel borders
        bg_stroke: Stroke::new(1.0, SEPARATOR),
        ..base
    };
    widgets.inactive = WidgetVisuals {
        bg_fill: BG_RAIL,
        ..base
    };
    widgets.hovered = WidgetVisuals {
        bg_fill: BG_WIDGET_ACTIVE,
        weak_bg_fill: BG_WIDGET_HOVERED,
        fg_stroke: Stroke::new(1.0, TEXT),
        ..base
    };
    widgets.active = WidgetVisuals {
        bg_fill: BG_WIDGET_ACTIVE,
        weak_bg_fill: BG_WIDGET_ACTIVE,
        fg_stroke: Stroke::new(1.0, TEXT),
        ..base
    };
    widgets.open = WidgetVisuals {
        bg_fill: BG_WIDGET,
        weak_bg_fill: BG_EXTREME,
        fg_stroke: Stroke::new(1.0, TEXT),
        ..base
    };

    style
}

/// Style of the menu bar and its menus: frameless buttons that only light up when hovered.
pub fn menu_style() -> egui::Style {
    let mut style = style();
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.item_spacing = egui::vec2(2.0, 1.0);
    let widgets = &mut style.visuals.widgets;
    widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    widgets.hovered.weak_bg_fill = BG_WIDGET_HOVERED;
    widgets.open.weak_bg_fill = BG_WIDGET_HOVERED;
    style
}
