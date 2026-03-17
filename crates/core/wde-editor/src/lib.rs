//! A simple editor plugin that demonstrates how to create a UI menu system using egui in Bevy.
//! The plugin defines a `UIMenu` resource that stores the structure of the menu and provides methods to add menu items and check their clicked state. The `EditorPlugin` initializes a demo menu with some common items and handles the "Help/About" menu item to show an about window when clicked.
//! 
//! # Example : The top-level menu
//! You can define terrain-related entries and gate a debug window on a clicked item:
//! ```rust,no_run
//! use bevy::prelude::*;
//! use wde_editor::prelude::*;
//!
//! fn init_ui(mut ui_menu: ResMut<UIMenu>) {
//!     ui_menu.push("Terrain/Paint");
//!     ui_menu.push("Terrain/Save");
//! }
//!
//! fn ui_paint_terrain(ctx: Res<UIContext>, ui_menu: Res<UIMenu>) {
//!     if !ui_menu.is_clicked("Terrain/Paint") {
//!         return;
//!     }
//!
//!     UIWindow::new("Paint Debug").show(&ctx.0, |_ui| {
//!         // Terrain paint controls...
//!     });
//! }
//! ```

use bevy::prelude::*;
use wde_egui::EguiPlugin;

use crate::{panels::PanelsPlugin, ui::EditorUIMenu};

mod ui;
mod panels;

pub mod prelude {
    pub mod ui {
        pub use wde_egui::prelude::*;
    }

    // Re-export egui types for easier access in editor code
    pub use wde_egui::prelude::egui::Window as UIWindow;
    pub use wde_egui::prelude::EguiContext as UIContext;

    // Re-export editor UI types
    pub use wde_egui::prelude::egui::{ComboBox, Slider, Checkbox, TextEdit, Button, Label, Separator, CollapsingHeader, ScrollArea, FontId, Color32, ColorImage, DragValue};

    // Re-export editor types
    pub use super::EditorPlugin;
    pub use super::ui::UIMenu;
}

pub struct EditorPlugin;
impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(EguiPlugin)
            .add_plugins(EditorUIMenu)
            .add_plugins(PanelsPlugin);
    }
}
