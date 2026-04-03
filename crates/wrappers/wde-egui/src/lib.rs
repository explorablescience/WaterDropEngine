//!! Egui integration for WaterDropEngine using Bevy.
//! 
//! This crate provides plugins and systems to integrate the egui immediate mode GUI library
//! with the WaterDropEngine, leveraging Bevy's ECS and rendering architecture.
//! 
//! # Features
//! - Egui context management
//! - Input handling from Bevy to egui
//! - Frame update and rendering integration
//! 
//! # Example
//! ```rust,no_run
//! fn draw_new_window(ctx: &EguiContext) {
//!   egui::Window::new("My Egui Window")
//!      .show(&ctx.0, |ui| {
//!        ui.label("Hello, egui in WaterDropEngine!");
//!      });
//! }
//! ```

pub mod prelude {
    pub use crate::EguiPlugin;
    pub use crate::egui::egui_context::EguiContext;
    pub use crate::egui::egui_pass::EguiRenderPassHolder;
    pub mod egui {
        pub use egui::*;
    }
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
