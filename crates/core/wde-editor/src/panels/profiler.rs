use bevy::prelude::*;
use puffin_egui::puffin;
use wde_egui::prelude::{EguiContext, egui};

use crate::ui::UIMenu;

pub struct ProfilerPlugin;
impl Plugin for ProfilerPlugin {
    fn build(&self, app: &mut App) {
        // Init puffin profiler
        puffin::set_scopes_on(true);

        // Add profiler
        app
            .add_systems(Startup, init_ui)
            .add_systems(Update, draw_profiler_panel);
    }
}

fn init_ui(mut ui_menu: ResMut<UIMenu>) {
    ui_menu.push("Engine/Profiler");
}

fn draw_profiler_panel(ctx: Res<EguiContext>, mut ui_menu: ResMut<UIMenu>) {
    let show_panel = ui_menu.is_clicked("Engine/Profiler");
    if !show_panel {
        return;
    }

    // Draw the profiler window using puffin_egui
    egui::Window::new("Profiler")
		.default_size([1100.0, 600.0])
        .open(ui_menu.clicked_mut("Engine/Profiler").unwrap_or(&mut false))
        .show(&ctx.0, |ui| {
            puffin_egui::profiler_ui(ui);
        });
}
