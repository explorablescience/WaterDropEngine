//! A simple editor plugin on top of `wde_egui` that provides an editor interface.
//!
//! # Example
//! To create as an example a top-level menu item "Terrain" with a submenu "Paint", you can do the following:
//! ```rust
//! fn update(ctx: Res<UIContext>, ui_menu: Res<UIMenu>) {
//!     UIWindow::new("Paint Debug")
//!         .default_size([1100.0, 600.0])
//!         .open(ui_menu.clicked_mut("Terrain/Paint"))
//!         .show(&ctx.0, |_ui| {
//!              // Terrain paint functionality goes here...
//!         });
//! }
//! ```
//! To see all the available UI components and how to use them, check out the [`egui`] documentation, which is the underlying UI library used by `wde_egui` and thus also by this editor plugin.
//!
//! # Default Panels
//! This crate also provides basic panels such as ones for inspecting entities and assets, as well as a simple overlay system for drawing UI on top of the game viewport.

use bevy::prelude::*;
use wde_egui::EguiPlugin;

use crate::{panels::PanelsPlugin, ui::EditorUIMenu};

mod panels;
mod ui;
mod ui_textures;

#[doc(hidden)]
pub mod prelude {
    // Re-export egui types for easier access in editor code
    pub use wde_egui::prelude::EguiContext as UIContext;
    pub use wde_egui::prelude::egui::Window as UIWindow;

    // Re-export editor UI types
    pub use wde_egui::prelude::egui::{
        Button, Checkbox, CollapsingHeader, Color32, ColorImage, ComboBox, DragValue, FontId,
        Label, ScrollArea, Separator, Slider, TextEdit
    };
    pub mod ui {
        pub use wde_egui::prelude::*;
    }

    // Re-export editor types
    pub use super::panels::FrameDataOverlayConfig;
    pub use super::ui::{EditorMenuBarSet, EngineUiConfig, EngineUiSet, UIMenu};
    pub use super::ui_textures::{UITextureHandle, UITextures};
}

pub struct EditorPlugin;
impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((EguiPlugin, EditorUIMenu, PanelsPlugin));
    }
}
