use std::path::Path;

use bevy::prelude::*;
use wde_egui::prelude::{EguiContext, egui};

use crate::ui::UIMenu;

#[derive(Resource, Default)]
pub struct AssetCatalog {
	paths: Vec<String>,
}

pub fn init_asset_catalog(mut asset_catalog: ResMut<AssetCatalog>) {
	let mut paths = Vec::new();
	collect_asset_paths(Path::new("res"), Path::new("res"), &mut paths);
	paths.sort();
	asset_catalog.paths = paths;
}
fn collect_asset_paths(root: &Path, current: &Path, out: &mut Vec<String>) {
	let Ok(entries) = std::fs::read_dir(current) else {
		return;
	};

	for entry in entries.flatten() {
		let path = entry.path();

		if path.is_dir() {
			collect_asset_paths(root, &path, out);
			continue;
		}

		let Ok(relative) = path.strip_prefix(root) else {
			continue;
		};

		let relative = relative.to_string_lossy().replace('\\', "/");
		out.push(relative);
	}
}


pub fn draw_assets_panel(
	ctx: Res<EguiContext>,
	ui_menu: Res<UIMenu>,
	asset_server: Res<AssetServer>,
	asset_catalog: Res<AssetCatalog>,
) {
	if !ui_menu.is_clicked("Engine/Assets") {
		return;
	}

	let mut rows = asset_catalog
		.paths
		.iter()
		.filter_map(|path| {
			let id = asset_server.get_path_id(path)?;
			if !asset_server.is_loaded_with_dependencies(id) {
				return None;
			}

			Some((id, path.clone()))
		})
		.collect::<Vec<_>>();
	rows.sort_by_key(|(_, path)| path.clone());

	egui::Window::new("ECS - Loaded Assets")
		.default_size([460.0, 440.0])
		.show(&ctx.0, |ui| {
			ui.label(format!("Total loaded assets: {}", rows.len()));
			ui.label("Shows loaded file-backed assets under res/.");
			ui.separator();

			egui::ScrollArea::vertical().show(ui, |ui| {
				for (id, path) in &rows {
					ui.label(format!("{:?} - {}", id, path));
				}
			});
		});
}
