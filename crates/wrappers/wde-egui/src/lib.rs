//! Egui integration for WaterDropEngine.
//!
//! # Example
//! To draw a new egui window, you can use the [`EguiContext`](crate::egui::egui_context::EguiContext) resource provided by the plugin:
//! ```rust
//! fn draw_new_window(ctx: &EguiContext) {
//!   egui::Window::new("My Egui Window")
//!      .show(&ctx.0, |ui| {
//!        ui.label("Hello, egui in WaterDropEngine!");
//!      });
//! }
//! ```

#[doc(hidden)]
pub mod prelude {
    pub use crate::egui::egui_context::EguiContext;
    pub use crate::logic::EguiInputSet;
    pub mod egui {
        pub use crate::egui::egui_pass::EguiRenderPassHolder;
        pub use egui::*;
    }
}

mod egui;
mod logic;

use bevy::prelude::*;

pub struct EguiPlugin;
impl Plugin for EguiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((egui::EguiLogicPlugin, logic::EguiRenderPlugin));
    }
}
