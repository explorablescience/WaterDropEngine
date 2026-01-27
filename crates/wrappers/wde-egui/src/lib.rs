pub mod prelude {
    pub use crate::EguiPlugin;
}

mod egui;
mod logic;

use bevy::prelude::*;

pub struct EguiPlugin;
impl Plugin for EguiPlugin {
    fn build(&self, app: &mut App) {
        // Initialize egui logic and rendering plugins
        app
            .add_plugins(egui::EguiLogicPlugin)
            .add_plugins(logic::EguiRenderPlugin);
    }
}
