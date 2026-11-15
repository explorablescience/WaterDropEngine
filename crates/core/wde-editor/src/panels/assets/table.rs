//! Virtualized table with sortable columns, shared by the tabs of the `Assets` window.
use std::sync::Arc;

use wde_egui::prelude::*;

const ROW_HEIGHT: f32 = 20.0;
const PADDING: f32 = 6.0;
const ARROW_SIZE: f32 = 7.0;

pub(super) struct Column {
    pub title: &'static str,
    /// `None` takes the remaining width, split between such columns.
    pub width: Option<f32>,
    pub right: bool
}
impl Column {
    pub const fn flex(title: &'static str) -> Self {
        Self {
            title,
            width: None,
            right: false
        }
    }
    pub const fn fixed(title: &'static str, width: f32) -> Self {
        Self {
            title,
            width: Some(width),
            right: false
        }
    }
    pub const fn right(title: &'static str, width: f32) -> Self {
        Self {
            title,
            width: Some(width),
            right: true
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Sort {
    pub column: usize,
    pub ascending: bool
}
impl Sort {
    pub const fn new(column: usize, ascending: bool) -> Self {
        Self { column, ascending }
    }
    /// Applies the direction to an ascending ordering.
    pub fn order(self, ordering: std::cmp::Ordering) -> std::cmp::Ordering {
        if self.ascending {
            ordering
        } else {
            ordering.reverse()
        }
    }
}

pub(super) struct Cell {
    pub text: String,
    pub color: Option<egui::Color32>,
    pub monospace: bool,
    /// Draws a dot of the color instead of the text.
    pub dot: bool
}
impl Cell {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: None,
            monospace: false,
            dot: false
        }
    }
    pub fn dot(color: egui::Color32) -> Self {
        Self {
            dot: true,
            ..Self::new("").color(color)
        }
    }
    pub fn mono(text: impl Into<String>) -> Self {
        Self {
            monospace: true,
            ..Self::new(text)
        }
    }
    pub fn color(mut self, color: egui::Color32) -> Self {
        self.color = Some(color);
        self
    }
    pub fn weak(self, ui: &egui::Ui) -> Self {
        let color = ui.visuals().weak_text_color();
        self.color(color)
    }
}

pub(super) struct Table<'a> {
    pub id: &'a str,
    pub columns: &'a [Column],
    pub rows: usize,
    pub selected: Option<usize>
}

pub(super) struct TableResponse {
    pub clicked: Option<usize>,
    pub hovered: Option<(usize, egui::Response)>
}

impl Table<'_> {
    /// Draws the header, toggling the `sort` when clicked, and the visible rows.
    pub fn show(
        self,
        ui: &mut egui::Ui,
        sort: &mut Sort,
        mut cell: impl FnMut(&egui::Ui, usize, usize) -> Cell
    ) -> TableResponse {
        let widths = self.widths(ui.available_width());
        self.header(ui, &widths, sort);

        let mut response = TableResponse {
            clicked: None,
            hovered: None
        };
        if self.rows == 0 {
            ui.add_space(4.0);
            ui.vertical_centered(|ui| ui.weak("Nothing to show"));
            return response;
        }
        egui::ScrollArea::vertical()
            .id_salt(self.id)
            .auto_shrink([false, false])
            .show_rows(ui, ROW_HEIGHT, self.rows, |ui, range| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for row in range {
                    let (rect, row_response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), ROW_HEIGHT),
                        egui::Sense::click()
                    );
                    let visuals = ui.visuals();
                    let background = if self.selected == Some(row) {
                        Some(visuals.selection.bg_fill)
                    } else if row_response.hovered() {
                        Some(visuals.widgets.hovered.weak_bg_fill)
                    } else if row % 2 == 1 {
                        Some(visuals.faint_bg_color)
                    } else {
                        None
                    };
                    if let Some(color) = background {
                        ui.painter().rect_filled(rect, 2.0, color);
                    }
                    let default_color = if self.selected == Some(row) {
                        visuals.selection.stroke.color
                    } else {
                        visuals.text_color()
                    };
                    let mut x = rect.left();
                    for (i, (column, width)) in self.columns.iter().zip(&widths).enumerate() {
                        let cell_rect = egui::Rect::from_min_size(
                            egui::pos2(x, rect.top()),
                            egui::vec2(*width, ROW_HEIGHT)
                        );
                        let content = cell(ui, row, i);
                        if content.dot {
                            let center =
                                egui::pos2(cell_rect.left() + PADDING + 4.0, cell_rect.center().y);
                            ui.painter().circle_filled(
                                center,
                                4.0,
                                content.color.unwrap_or(default_color)
                            );
                            x += width;
                            continue;
                        }
                        let galley = layout(
                            ui,
                            content.text,
                            content.monospace,
                            content.color.unwrap_or(default_color),
                            width - 2.0 * PADDING
                        );
                        paint(ui, cell_rect, galley, column.right);
                        x += width;
                    }
                    if row_response.clicked() {
                        response.clicked = Some(row);
                    }
                    if row_response.hovered() {
                        response.hovered = Some((row, row_response));
                    }
                }
            });
        response
    }

    fn header(&self, ui: &mut egui::Ui, widths: &[f32], sort: &mut Sort) {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), ROW_HEIGHT),
            egui::Sense::hover()
        );
        let mut x = rect.left();
        for (i, (column, width)) in self.columns.iter().zip(widths).enumerate() {
            let cell_rect = egui::Rect::from_min_size(
                egui::pos2(x, rect.top()),
                egui::vec2(*width, ROW_HEIGHT)
            );
            let response = ui.interact(
                cell_rect,
                ui.id().with((self.id, "header", i)),
                egui::Sense::click()
            );
            if response.clicked() {
                *sort = if sort.column == i {
                    Sort::new(i, !sort.ascending)
                } else {
                    // Sizes and counts are more useful largest first
                    Sort::new(i, !column.right)
                };
            }
            let visuals = ui.visuals();
            let color = if response.hovered() || sort.column == i {
                visuals.strong_text_color()
            } else {
                visuals.weak_text_color()
            };
            let sorted = sort.column == i && !column.title.is_empty();
            let arrow_width = if sorted { ARROW_SIZE + 4.0 } else { 0.0 };
            let galley = layout(
                ui,
                column.title.to_string(),
                false,
                color,
                width - 2.0 * PADDING - arrow_width
            );
            let text_width = galley.size().x;
            let text_rect = match column.right {
                true => cell_rect.with_max_x(cell_rect.right() - arrow_width),
                false => cell_rect
            };
            paint(ui, text_rect, galley, column.right);
            if sorted {
                let x = match column.right {
                    true => cell_rect.right() - PADDING - ARROW_SIZE / 2.0,
                    false => cell_rect.left() + PADDING + text_width + 4.0 + ARROW_SIZE / 2.0
                };
                sort_arrow(
                    ui,
                    egui::pos2(x, cell_rect.center().y),
                    sort.ascending,
                    color
                );
            }
            x += width;
        }
        let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
        ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
    }

    fn widths(&self, available: f32) -> Vec<f32> {
        let fixed: f32 = self.columns.iter().filter_map(|c| c.width).sum();
        let flex = self
            .columns
            .iter()
            .filter(|c| c.width.is_none())
            .count()
            .max(1);
        let flex_width = ((available - fixed) / flex as f32).max(60.0);
        self.columns
            .iter()
            .map(|c| c.width.unwrap_or(flex_width))
            .collect()
    }
}

fn layout(
    ui: &egui::Ui,
    text: String,
    monospace: bool,
    color: egui::Color32,
    width: f32
) -> Arc<egui::Galley> {
    let font = if monospace {
        egui::TextStyle::Monospace
    } else {
        egui::TextStyle::Body
    }
    .resolve(ui.style());
    let mut job = egui::text::LayoutJob::simple_singleline(text, font, color);
    job.wrap = egui::text::TextWrapping {
        max_width: width.max(0.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…')
    };
    ui.painter().layout_job(job)
}

fn paint(ui: &egui::Ui, rect: egui::Rect, galley: Arc<egui::Galley>, right: bool) {
    let x = if right {
        rect.right() - PADDING - galley.size().x
    } else {
        rect.left() + PADDING
    };
    let pos = egui::pos2(x, rect.center().y - galley.size().y / 2.0);
    ui.painter()
        .with_clip_rect(rect)
        .galley(pos, galley, egui::Color32::PLACEHOLDER);
}

fn sort_arrow(ui: &egui::Ui, center: egui::Pos2, up: bool, color: egui::Color32) {
    let (h, w) = (ARROW_SIZE * 0.6, ARROW_SIZE / 2.0);
    let (tip, base) = match up {
        true => (center.y - h / 2.0, center.y + h / 2.0),
        false => (center.y + h / 2.0, center.y - h / 2.0)
    };
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(center.x, tip),
            egui::pos2(center.x + w, base),
            egui::pos2(center.x - w, base),
        ],
        color,
        egui::Stroke::NONE
    ));
}

/// Bytes with a binary unit.
pub(super) fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    match unit {
        0 => format!("{bytes} B"),
        _ => format!("{value:.1} {}", UNITS[unit])
    }
}

/// Colored dot followed by a label.
pub(super) fn dot_label(
    ui: &mut egui::Ui,
    color: egui::Color32,
    text: impl Into<egui::WidgetText>
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 4.0, color);
        ui.label(text);
    });
}
