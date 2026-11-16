//! `Files` tab: the files of `res/`, with their load state in the [`AssetServer`].
use std::{
    collections::{BTreeMap, HashMap},
    path::Path
};

use bevy::{
    asset::{AssetPath, LoadState, RecursiveDependencyLoadState, UntypedAssetId},
    prelude::*,
    tasks::{block_on, futures_lite::future::poll_once},
    utils::prelude::ShortName
};
use wde_egui::prelude::*;

use super::table::{Cell, Column, Sort, Table, dot_label, format_bytes};

const ROOT: &str = "res";
const COLUMNS: [Column; 5] = [
    Column::fixed("", 22.0),
    Column::flex("Name"),
    Column::fixed("Type", 120.0),
    Column::fixed("State", 150.0),
    Column::right("Size", 80.0)
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum FileStatus {
    Failed,
    Loading,
    DepsLoading,
    Loaded,
    NotLoaded
}
impl FileStatus {
    const ALL: [Self; 5] = [
        Self::Loaded,
        Self::DepsLoading,
        Self::Loading,
        Self::Failed,
        Self::NotLoaded
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Failed => "Failed",
            Self::Loading => "Loading",
            Self::DepsLoading => "Loading dependencies",
            Self::Loaded => "Loaded",
            Self::NotLoaded => "Not loaded"
        }
    }

    fn color(self, ui: &egui::Ui) -> egui::Color32 {
        match self {
            Self::Failed => egui::Color32::from_rgb(230, 100, 90),
            Self::Loading | Self::DepsLoading => egui::Color32::from_rgb(225, 180, 80),
            Self::Loaded => egui::Color32::from_rgb(110, 190, 120),
            Self::NotLoaded => ui.visuals().weak_text_color()
        }
    }

    fn is_loaded(self) -> bool {
        matches!(self, Self::Loaded | Self::DepsLoading)
    }
}

struct CatalogFile {
    /// Relative to the [`ROOT`], as loaded by the asset server.
    path: String,
    asset_path: AssetPath<'static>,
    /// Folder of the file, relative to the [`ROOT`], empty at its root.
    folder: String,
    bytes: u64,
    /// Asset loaded by the loader of its extension.
    asset_type: Option<String>
}

struct FileState {
    status: FileStatus,
    error: Option<String>,
    id: Option<UntypedAssetId>
}

#[derive(Default)]
struct FolderNode {
    children: BTreeMap<String, FolderNode>
}

#[derive(Default, Clone, Copy)]
struct FolderCount {
    total: usize,
    loaded: usize,
    failed: usize
}

pub(super) struct FilesTab {
    files: Vec<CatalogFile>,
    tree: FolderNode,
    types: Vec<String>,
    /// Selected folder, empty for the [`ROOT`].
    folder: String,
    search: String,
    status: Option<FileStatus>,
    asset_type: Option<String>,
    /// Also lists the files no loader can load.
    unloadable: bool,
    sort: Sort,
    selected: Option<String>
}
impl Default for FilesTab {
    fn default() -> Self {
        Self {
            files: vec![],
            tree: FolderNode::default(),
            types: vec![],
            folder: String::new(),
            search: String::new(),
            status: None,
            asset_type: None,
            unloadable: false,
            sort: Sort::new(1, true),
            selected: None
        }
    }
}

impl FilesTab {
    /// Lists the files of the [`ROOT`] and the asset types of their loaders.
    pub fn scan(&mut self, asset_server: &AssetServer) {
        let mut paths = vec![];
        collect_files(Path::new(ROOT), Path::new(ROOT), &mut paths);
        let mut loaders = HashMap::new();
        self.files = paths
            .into_iter()
            .map(|(path, bytes)| {
                let (folder, name) = path.rsplit_once('/').unwrap_or(("", &path));
                let asset_type = name.split_once('.').and_then(|(_, extension)| {
                    loaders
                        .entry(extension.to_string())
                        .or_insert_with(|| loader_asset_type(asset_server, extension))
                        .clone()
                });
                CatalogFile {
                    folder: folder.to_string(),
                    asset_path: AssetPath::from(path.clone()),
                    path,
                    bytes,
                    asset_type
                }
            })
            .collect();

        self.tree = FolderNode::default();
        for file in self.files.iter().filter(|f| !f.folder.is_empty()) {
            let mut node = &mut self.tree;
            for part in file.folder.split('/') {
                node = node.children.entry(part.to_string()).or_default();
            }
        }
        self.types = self
            .files
            .iter()
            .filter_map(|f| f.asset_type.clone())
            .collect();
        self.types.sort();
        self.types.dedup();
    }

    pub fn show(&mut self, ui: &mut egui::Ui, asset_server: &AssetServer) {
        let states: Vec<FileState> = self
            .files
            .iter()
            .map(|f| file_state(asset_server, f.asset_path.clone()))
            .collect();
        let unloadable = self.unloadable;
        let listed = |file: &CatalogFile| unloadable || file.asset_type.is_some();

        // Counts of the listed files
        let mut status_counts = HashMap::<FileStatus, usize>::new();
        let mut folders = HashMap::<String, FolderCount>::new();
        let (mut disk_bytes, mut loaded_bytes, mut total) = (0, 0, 0);
        for (file, state) in self.files.iter().zip(&states).filter(|(f, _)| listed(f)) {
            total += 1;
            *status_counts.entry(state.status).or_default() += 1;
            disk_bytes += file.bytes;
            if state.status.is_loaded() {
                loaded_bytes += file.bytes;
            }
            for folder in ancestors(&file.folder) {
                let count = folders.entry(folder.to_string()).or_default();
                count.total += 1;
                count.loaded += state.status.is_loaded() as usize;
                count.failed += (state.status == FileStatus::Failed) as usize;
            }
        }

        self.toolbar(ui, asset_server);
        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(self.status.is_none(), format!("All {total}"))
                .clicked()
            {
                self.status = None;
            }
            for status in FileStatus::ALL {
                let count = status_counts.get(&status).copied().unwrap_or(0);
                if count == 0 && self.status != Some(status) {
                    continue;
                }
                let text = egui::RichText::new(format!("{} {count}", status.label()))
                    .color(status.color(ui));
                if ui
                    .selectable_label(self.status == Some(status), text)
                    .clicked()
                {
                    self.status = (self.status != Some(status)).then_some(status);
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(format!(
                    "{} of {} loaded",
                    format_bytes(loaded_bytes),
                    format_bytes(disk_bytes)
                ))
                .on_hover_text("Size on disk of the loaded files");
            });
        });
        ui.add_space(2.0);

        egui::Panel::left("assets-files-folders")
            .resizable(true)
            .default_size(220.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        folder_ui(ui, &self.tree, "", ROOT, 0, &folders, &mut self.folder);
                    });
            });

        // Listed files, filtered and sorted
        let search = self.search.to_lowercase();
        let mut rows: Vec<usize> = (0..self.files.len())
            .filter(|i| {
                let (file, state) = (&self.files[*i], &states[*i]);
                listed(file)
                    && in_folder(&file.folder, &self.folder)
                    && self.status.is_none_or(|s| s == state.status)
                    && self
                        .asset_type
                        .as_ref()
                        .is_none_or(|t| file.asset_type.as_ref() == Some(t))
                    && (search.is_empty() || file.path.to_lowercase().contains(&search))
            })
            .collect();
        let sort = self.sort;
        rows.sort_by(|a, b| {
            let (fa, fb) = (&self.files[*a], &self.files[*b]);
            let ordering = match sort.column {
                0 | 3 => states[*a].status.cmp(&states[*b].status),
                2 => fa.asset_type.cmp(&fb.asset_type),
                4 => fa.bytes.cmp(&fb.bytes),
                _ => std::cmp::Ordering::Equal
            };
            sort.order(ordering.then_with(|| fa.path.cmp(&fb.path)))
        });

        let selected = self
            .selected
            .as_ref()
            .and_then(|path| self.files.iter().position(|f| &f.path == path));
        if let Some(index) = selected {
            egui::Panel::bottom("assets-files-details")
                .resizable(false)
                .show(ui, |ui| {
                    ui.add_space(4.0);
                    self.details(ui, asset_server, index, &states[index]);
                });
        }

        egui::CentralPanel::default().show(ui, |ui| {
            let prefix = match self.folder.is_empty() {
                true => 0,
                false => self.folder.len() + 1
            };
            let response = Table {
                id: "assets-files",
                columns: &COLUMNS,
                rows: rows.len(),
                selected: rows.iter().position(|i| Some(*i) == selected)
            }
            .show(ui, &mut self.sort, |ui, row, column| {
                let (file, state) = (&self.files[rows[row]], &states[rows[row]]);
                match column {
                    0 => Cell::dot(state.status.color(ui)),
                    1 => Cell::new(&file.path[prefix..]),
                    2 => match &file.asset_type {
                        Some(t) => Cell::new(t),
                        None => Cell::new("-").weak(ui)
                    },
                    3 => Cell::new(state.status.label()).color(state.status.color(ui)),
                    _ => Cell::mono(format_bytes(file.bytes))
                }
            });
            if let Some(row) = response.clicked {
                let path = &self.files[rows[row]].path;
                self.selected = (self.selected.as_ref() != Some(path)).then(|| path.clone());
            }
            if let Some((row, response)) = response.hovered {
                let (file, state) = (&self.files[rows[row]], &states[rows[row]]);
                response.on_hover_ui(|ui| {
                    ui.monospace(&file.path);
                    if let Some(error) = &state.error {
                        ui.colored_label(FileStatus::Failed.color(ui), error);
                    }
                });
            }
        });
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, asset_server: &AssetServer) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Search files")
                    .desired_width(220.0)
            );
            if !self.search.is_empty() && ui.small_button("✕").clicked() {
                self.search.clear();
            }
            egui::ComboBox::from_id_salt("assets-files-type")
                .selected_text(self.asset_type.as_deref().unwrap_or("All types"))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.asset_type, None, "All types");
                    for asset_type in &self.types {
                        ui.selectable_value(
                            &mut self.asset_type,
                            Some(asset_type.clone()),
                            asset_type
                        );
                    }
                });
            ui.checkbox(&mut self.unloadable, "Files without loader")
                .on_hover_text("Also list the files that no asset loader can load");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("⟳ Rescan")
                    .on_hover_text(format!("List the files of {ROOT}/ again"))
                    .clicked()
                {
                    self.scan(asset_server);
                }
            });
        });
    }

    fn details(
        &mut self,
        ui: &mut egui::Ui,
        asset_server: &AssetServer,
        index: usize,
        state: &FileState
    ) {
        let file = &self.files[index];
        ui.horizontal(|ui| {
            ui.strong(file.path.rsplit('/').next().unwrap_or(&file.path));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    self.selected = None;
                }
                if ui
                    .add_enabled(state.id.is_some(), egui::Button::new("⟳ Reload"))
                    .on_hover_text("Reload the file from disk")
                    .on_disabled_hover_text("The file is not loaded")
                    .clicked()
                {
                    asset_server.reload(file.asset_path.clone());
                }
                if ui.button("Copy path").clicked() {
                    ui.ctx().copy_text(file.path.clone());
                }
            });
        });
        egui::Grid::new("assets-files-details-grid")
            .num_columns(2)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                ui.weak("Path");
                ui.monospace(format!("{ROOT}/{}", file.path));
                ui.end_row();
                ui.weak("Type");
                ui.label(file.asset_type.as_deref().unwrap_or("No loader"));
                ui.end_row();
                ui.weak("State");
                dot_label(ui, state.status.color(ui), state.status.label());
                ui.end_row();
                ui.weak("Size on disk");
                ui.label(format_bytes(file.bytes));
                ui.end_row();
                if let Some(id) = state.id {
                    ui.weak("Asset id");
                    ui.monospace(match id {
                        UntypedAssetId::Index { index, .. } => format!("{index:?}"),
                        UntypedAssetId::Uuid { uuid, .. } => uuid.to_string()
                    });
                    ui.end_row();
                }
            });
        if let Some(error) = &state.error {
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .id_salt("assets-files-error")
                .max_height(80.0)
                .show(ui, |ui| {
                    ui.colored_label(FileStatus::Failed.color(ui), error);
                });
        }
        ui.add_space(4.0);
    }
}

fn collect_files(root: &Path, current: &Path, out: &mut Vec<(String, u64)>) {
    let Ok(entries) = std::fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, out);
        } else if let Ok(relative) = path.strip_prefix(root) {
            let bytes = std::fs::metadata(&path).map_or(0, |m| m.len());
            out.push((relative.to_string_lossy().replace('\\', "/"), bytes));
        }
    }
}

/// Short name of the asset loaded for an extension, trying `frag.wgsl` then `wgsl`.
fn loader_asset_type(asset_server: &AssetServer, mut extension: &str) -> Option<String> {
    loop {
        // Polled once: a loader still being registered would block
        let loader = block_on(poll_once(
            asset_server.get_asset_loader_with_extension(extension)
        ));
        if let Some(Ok(loader)) = loader {
            return Some(ShortName(loader.asset_type_name()).to_string());
        }
        extension = extension.split_once('.')?.1;
    }
}

fn file_state(asset_server: &AssetServer, path: AssetPath<'static>) -> FileState {
    let state = |status, error, id| FileState { status, error, id };
    let Some(id) = asset_server.get_path_id(path) else {
        return state(FileStatus::NotLoaded, None, None);
    };
    let Some((load, _, dependencies)) = asset_server.get_load_states(id) else {
        return state(FileStatus::NotLoaded, None, Some(id));
    };
    match (load, dependencies) {
        (LoadState::Failed(error), _) => {
            state(FileStatus::Failed, Some(error.to_string()), Some(id))
        }
        (LoadState::Loaded, RecursiveDependencyLoadState::Failed(error)) => state(
            FileStatus::Failed,
            Some(format!("A dependency failed to load: {error}")),
            Some(id)
        ),
        (LoadState::Loaded, RecursiveDependencyLoadState::Loaded) => {
            state(FileStatus::Loaded, None, Some(id))
        }
        (LoadState::Loaded, _) => state(FileStatus::DepsLoading, None, Some(id)),
        (LoadState::Loading, _) => state(FileStatus::Loading, None, Some(id)),
        (LoadState::NotLoaded, _) => state(FileStatus::NotLoaded, None, Some(id))
    }
}

/// The folder and its parents, up to the root (`""`).
fn ancestors(folder: &str) -> impl Iterator<Item = &str> {
    std::iter::once("").chain(
        folder
            .match_indices('/')
            .map(|(i, _)| &folder[..i])
            .chain((!folder.is_empty()).then_some(folder))
    )
}

fn in_folder(folder: &str, selected: &str) -> bool {
    selected.is_empty()
        || folder == selected
        || folder
            .strip_prefix(selected)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn folder_ui(
    ui: &mut egui::Ui,
    node: &FolderNode,
    path: &str,
    name: &str,
    depth: usize,
    counts: &HashMap<String, FolderCount>,
    selected: &mut String
) {
    let count = counts.get(path).copied().unwrap_or_default();
    let label = |ui: &mut egui::Ui, selected: &mut String| {
        let mut text = egui::RichText::new(name);
        if count.total == 0 {
            text = text.weak();
        }
        let response = ui
            .selectable_label(selected == path, text)
            .on_hover_text(format!(
                "{} of {} files loaded{}",
                count.loaded,
                count.total,
                match count.failed {
                    0 => String::new(),
                    n => format!(", {n} failed")
                }
            ));
        if response.clicked() {
            *selected = path.to_string();
        }
        let color = match count.failed {
            0 => ui.visuals().weak_text_color(),
            _ => FileStatus::Failed.color(ui)
        };
        ui.label(
            egui::RichText::new(format!("{}/{}", count.loaded, count.total))
                .small()
                .color(color)
        );
    };

    if node.children.is_empty() {
        ui.horizontal(|ui| {
            ui.add_space(ui.spacing().indent);
            label(ui, selected);
        });
        return;
    }
    let id = ui.make_persistent_id(("assets-folder", path));
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, depth == 0)
        .show_header(ui, |ui| label(ui, selected))
        .body(|ui| {
            for (child, child_node) in &node.children {
                let child_path = match path.is_empty() {
                    true => child.clone(),
                    false => format!("{path}/{child}")
                };
                folder_ui(
                    ui,
                    child_node,
                    &child_path,
                    child,
                    depth + 1,
                    counts,
                    selected
                );
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders() {
        assert_eq!(ancestors("").collect::<Vec<_>>(), [""]);
        assert_eq!(
            ancestors("a/b/c").collect::<Vec<_>>(),
            ["", "a", "a/b", "a/b/c"]
        );
        assert!(in_folder("models/rocks", "models"));
        assert!(in_folder("models", "models"));
        assert!(!in_folder("models2", "models"));
        assert!(in_folder("anything", ""));
    }
}
