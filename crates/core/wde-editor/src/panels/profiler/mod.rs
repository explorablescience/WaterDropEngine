use bevy::prelude::*;
use puffin_egui::puffin;
use wde_egui::prelude::EguiContext;

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

fn draw_profiler_panel(ctx: Res<EguiContext>, ui_menu: Res<UIMenu>) {
    let show_panel = ui_menu.is_clicked("Engine/Profiler");
    if !show_panel {
        return;
    }

    // Draw the profiler window using puffin_egui
    puffin_egui::profiler_window(&ctx.0);
}
