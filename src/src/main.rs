#![allow(clippy::just_underscores_and_digits)]
#![allow(clippy::type_complexity)]

use wde::prelude::*;
use bevy::prelude::*;

pub fn main() {
    // Create the app
    let mut app = App::new();

    // Add default plugins
    info!("Adding default plugins.");
    app
        .add_plugins(WdeDefaultPlugins)
        .add_plugins(TestPlugin);

    // Run the app
    info!("Running game engine.");
    app.run();
}

pub struct TestPlugin;
impl Plugin for TestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, test_window);
    }
}

/// System to update the egui context with new input and generate frame output
fn test_window(ctx: Res<EguiContext>) {
    egui::Window::new("winit + egui + wgpu says hello!")
        .resizable(true)
        .vscroll(true)
        .default_open(false)
        .show(&ctx.0, |ui| {
            ui.label("Label!");
            if ui.button("Button!").clicked() {
                println!("boom!")
            }
        });
}

