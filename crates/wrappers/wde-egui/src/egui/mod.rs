pub mod egui_context;
pub mod egui_inputs;
pub mod egui_pass;

use bevy::prelude::*;

use crate::egui::{egui_context::EguiContextPlugin, egui_inputs::EguiInputsPlugin, egui_pass::EguiRenderPassPlugin};

pub struct EguiLogicPlugin;
impl Plugin for EguiLogicPlugin {
    fn build(&self, app: &mut App) {
        // Add input and renderpass plugins
        app
            .add_plugins(EguiContextPlugin)
            .add_plugins(EguiInputsPlugin)
            .add_plugins(EguiRenderPassPlugin);
    }
}
