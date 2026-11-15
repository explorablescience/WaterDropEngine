//! `Engine/Assets` window: the asset files and their load state, the GPU memory of the buffers and textures, and the
//! render assets of the render world.
use bevy::prelude::*;
use wde_egui::prelude::*;
use wde_renderer::prelude::*;

use crate::prelude::*;

mod files;
mod gpu;
mod render_assets;
mod table;

pub struct AssetsPanelPlugin;
impl Plugin for AssetsPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetsPanelState>()
            .add_systems(PostStartup, scan_files)
            .add_systems(Update, draw_ui.in_set(EngineUiSet).after(EditorMenuBarSet));
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Tab {
    #[default]
    Files,
    GpuMemory,
    RenderAssets
}

#[derive(Resource, Default)]
struct AssetsPanelState {
    tab: Tab,
    files: files::FilesTab,
    gpu: gpu::GpuTab,
    render_assets: render_assets::RenderAssetsTab
}

fn scan_files(mut state: ResMut<AssetsPanelState>, asset_server: Res<AssetServer>) {
    state.files.scan(&asset_server);
}

fn draw_ui(
    ctx: Res<EguiContext>,
    mut ui_menu: ResMut<UIMenu>,
    mut state: ResMut<AssetsPanelState>,
    mut stats: ResMut<RenderStats>,
    asset_server: Res<AssetServer>,
    time: Res<Time>
) {
    let state = &mut *state;
    // Sampled while closed, to show the history when opened
    state.gpu.sample(time.elapsed_secs_f64());

    let open = ui_menu.clicked_mut("Engine/Assets");
    let driver_report = *open && state.tab == Tab::GpuMemory;
    if stats.driver_report != driver_report {
        stats.driver_report = driver_report;
    }
    if !*open {
        return;
    }

    egui::Window::new("Assets")
        .default_size([1100.0, 650.0])
        .resizable(true)
        .open(open)
        .show(&ctx.0, |ui| {
            ui.horizontal(|ui| {
                let gpu = match state.gpu.total() {
                    Some(bytes) => format!("GPU Memory  {}", table::format_bytes(bytes)),
                    None => "GPU Memory".to_string()
                };
                let render_assets = format!(
                    "Render Assets  {}",
                    stats.render_assets.values().map(|c| c.ready).sum::<usize>()
                );
                for (tab, label) in [
                    (Tab::Files, "Files".to_string()),
                    (Tab::GpuMemory, gpu),
                    (Tab::RenderAssets, render_assets)
                ] {
                    ui.selectable_value(&mut state.tab, tab, label);
                }
            });
            ui.separator();
            match state.tab {
                Tab::Files => state.files.show(ui, &asset_server),
                Tab::GpuMemory => state.gpu.show(ui, &stats),
                Tab::RenderAssets => state.render_assets.show(ui, &stats)
            }
            // The central panels do not count as used space, which would shrink the window
            ui.expand_to_include_rect(ui.max_rect());
        });
}
