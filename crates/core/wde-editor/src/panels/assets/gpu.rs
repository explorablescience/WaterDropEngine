//! `GPU Memory` tab: the memory of the buffers and textures tracked by [`GpuMemory`], by category.
use std::collections::VecDeque;

use wde_egui::prelude::*;
use wde_renderer::{prelude::*, wgpu_utils::*};

use super::table::{Cell, Column, Sort, Table, format_bytes};

/// Seconds between two samples of the history.
const SAMPLE_PERIOD: f64 = 0.25;
/// Samples of the history, a minute long.
const HISTORY: usize = 240;
const PLOT_HEIGHT: f32 = 80.0;
/// Space above the largest value, for the legend.
const PLOT_HEADROOM: f32 = 1.35;
const COLUMNS: [Column; 4] = [
    Column::flex("Label"),
    Column::fixed("Category", 150.0),
    Column::flex("Details"),
    Column::right("Size", 90.0)
];

#[derive(Clone, Copy, Default)]
struct Sample {
    textures: u64,
    buffers: u64
}

pub(super) struct GpuTab {
    search: String,
    category: Option<GpuMemoryCategory>,
    sort: Sort,
    history: VecDeque<Sample>,
    last_sample: Option<f64>
}
impl Default for GpuTab {
    fn default() -> Self {
        Self {
            search: String::new(),
            category: None,
            sort: Sort::new(3, false),
            history: VecDeque::with_capacity(HISTORY),
            last_sample: None
        }
    }
}

impl GpuTab {
    /// Records the totals in the history, every [`SAMPLE_PERIOD`].
    pub fn sample(&mut self, time: f64) {
        if self
            .last_sample
            .is_some_and(|last| time - last < SAMPLE_PERIOD)
        {
            return;
        }
        self.last_sample = Some(time);
        if self.history.len() == HISTORY {
            self.history.pop_front();
        }
        let (textures, buffers) = kind_totals(&GpuMemory::totals());
        self.history.push_back(Sample {
            textures: textures.bytes,
            buffers: buffers.bytes
        });
    }

    /// Total of the tracked memory.
    pub fn total(&self) -> Option<u64> {
        self.history.back().map(|s| s.textures + s.buffers)
    }

    pub fn show(&mut self, ui: &mut egui::Ui, stats: &RenderStats) {
        let totals = GpuMemory::totals();
        let (textures, buffers) = kind_totals(&totals);

        ui.horizontal(|ui| {
            stat_card(
                ui,
                "Total",
                textures.bytes + buffers.bytes,
                format!("{} resources", textures.count + buffers.count)
            );
            stat_card(
                ui,
                "Textures",
                textures.bytes,
                format!("{} textures", textures.count)
            );
            stat_card(
                ui,
                "Buffers",
                buffers.bytes,
                format!("{} buffers", buffers.count)
            );
            match stats.driver_memory {
                Some(driver) => stat_card(
                    ui,
                    "Driver allocator",
                    driver.allocated_bytes,
                    format!(
                        "{} reserved in {} blocks",
                        format_bytes(driver.reserved_bytes),
                        driver.blocks
                    )
                ),
                None => {
                    ui.weak("Driver allocator: n/a").on_hover_text(
                        "The backend does not report the memory of its allocator (only DX12 does)."
                    );
                }
            }
        });
        ui.add_space(4.0);
        self.history_plot(ui);
        ui.add_space(6.0);

        ui.columns(2, |columns| {
            for (ui, (title, kind, total)) in columns
                .iter_mut()
                .zip([("Textures", true, textures), ("Buffers", false, buffers)])
            {
                ui.strong(title);
                let categories: Vec<_> = totals
                    .iter()
                    .filter(|(c, _)| c.is_texture() == kind)
                    .copied()
                    .collect();
                category_bar(ui, &categories, total.bytes);
                self.category_legend(ui, title, &categories, total.bytes);
            }
        });
        ui.separator();

        self.resources(ui);
    }

    fn history_plot(&self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), PLOT_HEIGHT),
            egui::Sense::hover()
        );
        let painter = ui.painter_at(rect);
        let visuals = ui.visuals();
        painter.rect_filled(rect, 4.0, visuals.extreme_bg_color);
        let max = self
            .history
            .iter()
            .map(|s| s.textures + s.buffers)
            .max()
            .unwrap_or(0)
            .max(1) as f32
            * PLOT_HEADROOM;
        let point = |i: usize, value: u64| {
            egui::pos2(
                rect.left() + rect.width() * i as f32 / (HISTORY - 1) as f32,
                rect.bottom() - rect.height() * value as f32 / max
            )
        };
        // Right-aligned, the last sample at the right edge
        let offset = HISTORY - self.history.len();
        for (line, color) in [TEXTURE_COLOR, BUFFER_COLOR, visuals.text_color()]
            .into_iter()
            .enumerate()
        {
            let points: Vec<_> = self
                .history
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let value = match line {
                        0 => s.textures,
                        1 => s.buffers,
                        _ => s.textures + s.buffers
                    };
                    point(offset + i, value)
                })
                .collect();
            painter.add(egui::Shape::line(points, egui::Stroke::new(1.5, color)));
        }
        let font = egui::FontId::proportional(11.0);
        let mut legend = rect.right_top() + egui::vec2(-8.0, 4.0);
        for (label, color) in [
            ("Buffers", BUFFER_COLOR),
            ("Textures", TEXTURE_COLOR),
            ("Total", visuals.text_color())
        ] {
            let text = painter.text(legend, egui::Align2::RIGHT_TOP, label, font.clone(), color);
            legend.x = text.left() - 10.0;
        }
        painter.text(
            rect.left_top() + egui::vec2(6.0, 4.0),
            egui::Align2::LEFT_TOP,
            format_bytes((max / PLOT_HEADROOM) as u64),
            font.clone(),
            visuals.weak_text_color()
        );
        painter.text(
            rect.left_bottom() + egui::vec2(6.0, -4.0),
            egui::Align2::LEFT_BOTTOM,
            "1 min",
            font,
            visuals.weak_text_color()
        );

        let Some(pointer) = response.hover_pos() else {
            return;
        };
        let i = (((pointer.x - rect.left()) / rect.width() * (HISTORY - 1) as f32).round()
            as usize)
            .checked_sub(offset);
        let Some(sample) = i.and_then(|i| self.history.get(i)) else {
            return;
        };
        painter.vline(
            pointer.x,
            rect.y_range(),
            egui::Stroke::new(1.0, visuals.weak_text_color())
        );
        let ago = (self.history.len() - 1 - i.unwrap()) as f64 * SAMPLE_PERIOD;
        response.on_hover_text_at_pointer(format!(
            "{ago:.1} s ago\nTotal {}\nTextures {}\nBuffers {}",
            format_bytes(sample.textures + sample.buffers),
            format_bytes(sample.textures),
            format_bytes(sample.buffers)
        ));
    }

    fn category_legend(
        &mut self,
        ui: &mut egui::Ui,
        id: &str,
        categories: &[(GpuMemoryCategory, GpuMemoryTotal)],
        total: u64
    ) {
        egui::Grid::new(("assets-gpu-legend", id))
            .num_columns(4)
            .spacing([12.0, 2.0])
            .show(ui, |ui| {
                for (category, memory) in categories {
                    let selected = self.category == Some(*category);
                    ui.horizontal(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, 2.0, category_color(*category));
                        let mut text = egui::RichText::new(category.label());
                        if memory.count == 0 {
                            text = text.weak();
                        }
                        if ui
                            .selectable_label(selected, text)
                            .on_hover_text("Filter the resources by this category")
                            .clicked()
                        {
                            self.category = (!selected).then_some(*category);
                        }
                    });
                    ui.weak(memory.count.to_string());
                    ui.monospace(format_bytes(memory.bytes));
                    ui.weak(match total {
                        0 => "-".to_string(),
                        _ => format!("{:.0}%", memory.bytes as f64 / total as f64 * 100.0)
                    });
                    ui.end_row();
                }
            });
    }

    fn resources(&mut self, ui: &mut egui::Ui) {
        let allocations = GpuMemory::allocations();
        ui.horizontal(|ui| {
            ui.strong("Resources");
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Search labels")
                    .desired_width(200.0)
            );
            if !self.search.is_empty() && ui.small_button("✕").clicked() {
                self.search.clear();
            }
            egui::ComboBox::from_id_salt("assets-gpu-category")
                .selected_text(
                    self.category
                        .map_or("All categories".to_string(), category_name)
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.category, None, "All categories");
                    for category in GpuMemoryCategory::ALL {
                        ui.selectable_value(
                            &mut self.category,
                            Some(category),
                            category_name(category)
                        );
                    }
                });
        });

        let search = self.search.to_lowercase();
        let mut rows: Vec<&GpuAllocationInfo> = allocations
            .iter()
            .filter(|a| {
                self.category.is_none_or(|c| c == a.category)
                    && (search.is_empty() || a.label.to_lowercase().contains(&search))
            })
            .collect();
        let sort = self.sort;
        rows.sort_by(|a, b| {
            let ordering = match sort.column {
                0 => a.label.cmp(&b.label),
                1 => a.category.cmp(&b.category),
                2 => a.description.cmp(&b.description),
                _ => a.bytes.cmp(&b.bytes)
            };
            sort.order(ordering.then_with(|| a.label.cmp(&b.label)))
        });
        let shown: u64 = rows.iter().map(|a| a.bytes).sum();
        ui.weak(format!(
            "{} of {} resources, {}. Sizes are estimated from the descriptors, and the resources of the editor \
             UI are not tracked.",
            rows.len(),
            allocations.len(),
            format_bytes(shown)
        ));

        let response = Table {
            id: "assets-gpu",
            columns: &COLUMNS,
            rows: rows.len(),
            selected: None
        }
        .show(ui, &mut self.sort, |ui, row, column| {
            let allocation = rows[row];
            match column {
                0 => Cell::new(&allocation.label),
                1 => Cell::new(category_name(allocation.category))
                    .color(category_color(allocation.category)),
                2 => Cell::new(&allocation.description).weak(ui),
                _ => Cell::mono(format_bytes(allocation.bytes))
            }
        });
        if let Some((row, response)) = response.hovered {
            let allocation = rows[row];
            response.on_hover_text(format!(
                "{}\n{}{}\n{} bytes",
                allocation.label,
                category_name(allocation.category),
                match allocation.description.is_empty() {
                    true => String::new(),
                    false => format!(", {}", allocation.description)
                },
                allocation.bytes
            ));
        }
    }
}

const TEXTURE_COLOR: egui::Color32 = egui::Color32::from_rgb(100, 150, 235);
const BUFFER_COLOR: egui::Color32 = egui::Color32::from_rgb(235, 155, 75);

fn category_color(category: GpuMemoryCategory) -> egui::Color32 {
    use GpuMemoryCategory as C;
    let (r, g, b) = match category {
        C::SampledTexture => (100, 150, 235),
        C::RenderTarget => (160, 115, 235),
        C::DepthStencil => (80, 195, 205),
        C::StorageTexture => (215, 120, 205),
        C::VertexBuffer => (235, 155, 75),
        C::IndexBuffer => (225, 195, 85),
        C::IndirectBuffer => (230, 100, 90),
        C::StorageBuffer => (130, 195, 105),
        C::UniformBuffer => (105, 175, 155),
        C::StagingBuffer => (175, 175, 175),
        C::OtherBuffer => (125, 125, 125)
    };
    egui::Color32::from_rgb(r, g, b)
}

fn category_name(category: GpuMemoryCategory) -> String {
    let kind = match category.is_texture() {
        true => "texture",
        false => "buffer"
    };
    format!("{} {kind}", category.label())
}

fn kind_totals(totals: &[(GpuMemoryCategory, GpuMemoryTotal)]) -> (GpuMemoryTotal, GpuMemoryTotal) {
    let mut textures = GpuMemoryTotal::default();
    let mut buffers = GpuMemoryTotal::default();
    for (category, total) in totals {
        let kind = match category.is_texture() {
            true => &mut textures,
            false => &mut buffers
        };
        kind.count += total.count;
        kind.bytes += total.bytes;
    }
    (textures, buffers)
}

fn stat_card(ui: &mut egui::Ui, title: &str, bytes: u64, detail: String) {
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_min_width(130.0);
            ui.vertical(|ui| {
                ui.weak(title);
                ui.label(egui::RichText::new(format_bytes(bytes)).heading().strong());
                ui.weak(detail);
            });
        });
}

/// Bar split between the categories, by size.
fn category_bar(ui: &mut egui::Ui, categories: &[(GpuMemoryCategory, GpuMemoryTotal)], total: u64) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 12.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, ui.visuals().extreme_bg_color);
    if total == 0 {
        return;
    }
    let mut x = rect.left();
    let mut hovered = None;
    for (category, memory) in categories.iter().filter(|(_, m)| m.bytes > 0) {
        let width = rect.width() * (memory.bytes as f64 / total as f64) as f32;
        let segment =
            egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(width, rect.height()));
        painter.rect_filled(segment, 0.0, category_color(*category));
        if response.hover_pos().is_some_and(|p| segment.contains(p)) {
            hovered = Some((*category, *memory));
        }
        x += width;
    }
    if let Some((category, memory)) = hovered {
        response.on_hover_text_at_pointer(format!(
            "{}: {} in {} resources",
            category_name(category),
            format_bytes(memory.bytes),
            memory.count
        ));
    }
}
