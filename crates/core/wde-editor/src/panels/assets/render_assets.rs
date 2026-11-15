//! `Render Assets` tab: the render assets of each type in the render world (see [`RenderStats`]).
use wde_egui::prelude::*;
use wde_renderer::prelude::*;

use super::table::{Cell, Column, Sort, Table};

const COLUMNS: [Column; 4] = [
    Column::flex("Type"),
    Column::fixed("Kind", 120.0),
    Column::right("Ready", 80.0),
    Column::right("Pending", 80.0)
];
const KINDS: [RenderAssetKind; 4] = [
    RenderAssetKind::Asset,
    RenderAssetKind::Pipeline,
    RenderAssetKind::RenderData,
    RenderAssetKind::RenderBinding
];
const PENDING_COLOR: egui::Color32 = egui::Color32::from_rgb(225, 180, 80);

pub(super) struct RenderAssetsTab {
    search: String,
    kind: Option<RenderAssetKind>,
    /// Hides the types without render assets.
    hide_empty: bool,
    sort: Sort
}
impl Default for RenderAssetsTab {
    fn default() -> Self {
        Self {
            search: String::new(),
            kind: None,
            hide_empty: false,
            sort: Sort::new(1, true)
        }
    }
}

impl RenderAssetsTab {
    pub fn show(&mut self, ui: &mut egui::Ui, stats: &RenderStats) {
        let counts: Vec<&RenderAssetCount> = stats.render_assets.values().collect();

        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(self.kind.is_none(), format!("All {}", counts.len()))
                .clicked()
            {
                self.kind = None;
            }
            for kind in KINDS {
                let (types, ready) = counts
                    .iter()
                    .filter(|c| c.kind == kind)
                    .fold((0, 0), |(t, r), c| (t + 1, r + c.ready));
                if ui
                    .selectable_label(
                        self.kind == Some(kind),
                        format!("{} {types}", kind_label(kind))
                    )
                    .on_hover_text(format!("{ready} ready in {types} types"))
                    .clicked()
                {
                    self.kind = (self.kind != Some(kind)).then_some(kind);
                }
            }
            let pending: usize = counts.iter().map(|c| c.pending).sum();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let ready: usize = counts.iter().map(|c| c.ready).sum();
                if pending > 0 {
                    ui.colored_label(PENDING_COLOR, format!("{pending} pending"))
                        .on_hover_text("Waiting for their dependencies to be prepared again");
                }
                ui.weak(format!("{ready} ready"));
            });
        });
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Search types")
                    .desired_width(220.0)
            );
            if !self.search.is_empty() && ui.small_button("✕").clicked() {
                self.search.clear();
            }
            ui.checkbox(&mut self.hide_empty, "Hide empty types");
        });

        let search = self.search.to_lowercase();
        let mut rows: Vec<&RenderAssetCount> = counts
            .into_iter()
            .filter(|c| {
                self.kind.is_none_or(|k| k == c.kind)
                    && (!self.hide_empty || c.ready + c.pending > 0)
                    && (search.is_empty() || c.name.to_lowercase().contains(&search))
            })
            .collect();
        let sort = self.sort;
        rows.sort_by(|a, b| {
            let ordering = match sort.column {
                1 => a.kind.cmp(&b.kind),
                2 => a.ready.cmp(&b.ready),
                3 => a.pending.cmp(&b.pending),
                _ => std::cmp::Ordering::Equal
            };
            sort.order(ordering.then_with(|| a.name.cmp(&b.name)))
        });

        let response = Table {
            id: "assets-render",
            columns: &COLUMNS,
            rows: rows.len(),
            selected: None
        }
        .show(ui, &mut self.sort, |ui, row, column| {
            let count = rows[row];
            match column {
                0 => Cell::new(&count.name),
                1 => Cell::new(kind_label(count.kind)).weak(ui),
                2 => match count.ready {
                    0 => Cell::mono("0").weak(ui),
                    n => Cell::mono(n.to_string())
                },
                _ => match count.pending {
                    0 => Cell::mono("0").weak(ui),
                    n => Cell::mono(n.to_string()).color(PENDING_COLOR)
                }
            }
        });
        if let Some((row, response)) = response.hovered {
            response.on_hover_text(rows[row].type_path);
        }
    }
}

fn kind_label(kind: RenderAssetKind) -> &'static str {
    match kind {
        RenderAssetKind::Asset => "Assets",
        RenderAssetKind::Pipeline => "Pipelines",
        RenderAssetKind::RenderData => "Render data",
        RenderAssetKind::RenderBinding => "Bindings"
    }
}
