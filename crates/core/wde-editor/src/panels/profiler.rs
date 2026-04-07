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
        app.add_systems(Update, draw_profiler_panel);
    }
}

fn draw_profiler_panel(ctx: Res<EguiContext>, mut ui_menu: ResMut<UIMenu>) {
    egui::Window::new("Profiler")
        .default_size([1100.0, 600.0])
        .open(ui_menu.clicked_mut("Engine/Profiler"))
        .show(&ctx.0, |ui| {
            puffin_egui::profiler_ui(ui);
        });
}
