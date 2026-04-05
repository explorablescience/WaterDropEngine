use std::{any::TypeId, path::Path};

use wde_renderer::prelude::*;
use wde_egui::prelude::{EguiContext, egui};
use bevy::prelude::*;

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
	mut ui_menu: ResMut<UIMenu>,
	asset_server: Res<AssetServer>,
	asset_catalog: Res<AssetCatalog>,
) {
	if !ui_menu.is_clicked("Engine/Assets") {
		return;
	}

    // Get list of loaded assets and all assets
    let loading_assets = get_assets(&asset_server, &asset_catalog, false);
	let loaded_assets = get_assets(&asset_server, &asset_catalog, true);

    // Show the assets panel
	egui::Window::new("Assets")
		.default_size([1100.0, 600.0])
        .open(ui_menu.clicked_mut("Engine/Assets").unwrap_or(&mut false))
		.show(&ctx.0, |ui| {
            // Loaded assets
            ui.heading("Loaded Assets");
			ui.label(format!("Total: {}", loaded_assets.len()));
            ui.spacing();
			egui::ScrollArea::vertical()
                .id_salt("loaded")
                .show(ui, |ui| {
                    for (id, path) in &loaded_assets {
                        ui.label(format!("{:?} - {}", id, path));
                    }
                });

            ui.separator();

            // Loading assets
            ui.heading("Loading Assets");
            ui.label(format!("Total: {}", loading_assets.len()));
            ui.spacing();
            egui::ScrollArea::vertical()
                .id_salt("loading")
                .show(ui, |ui| {
                    for (id, path) in &loading_assets {
                        ui.label(format!("{:?} - {}", id, path));
                    }
                });
		});
}

fn get_assets(
    asset_server: &AssetServer,
    asset_catalog: &AssetCatalog,
    only_loaded: bool,
) -> Vec<(String, String)> {
    // Collect assets based on the catalog and filter by loaded status
    let mut assets = asset_catalog
        .paths
        .iter()
        .filter_map(|path| {
            let id = asset_server.get_path_id(path)?;
            if asset_server.is_loaded_with_dependencies(id) != only_loaded {
                return None;
            }
            let type_id = id.type_id();
            let mut type_name = "Unknown";
            if type_id == TypeId::of::<Mesh3d>() {
                type_name = "Mesh";
            // } else if type_id == TypeId::of::<dyn Material>() {
            //     type_name = "Material";
            } else if type_id == TypeId::of::<Texture>() {
                type_name = "Texture";
            } else if type_id == TypeId::of::<Shader>() {
                type_name = "Shader";
            }
            Some((type_name.to_string(), path.clone()))
        })
        .collect::<Vec<_>>();

    // Sort assets by type and path
    assets.sort_by(|a, b| {
        a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1))
    });

    assets
}
