//! A simple editor plugin on top of `wde_egui` that provides an editor interface.
//!
//! # Example
//! To create as an example a top-level menu item "Terrain" with a submenu "Paint", you can do the following:
//! ```rust
//! # use bevy::prelude::*;
//! # use wde_editor::prelude::*;
//! fn update(ctx: Res<UIContext>, mut ui_menu: ResMut<UIMenu>) {
//!     UIWindow::new("Paint Debug")
//!         .open(ui_menu.clicked_mut("Terrain/Paint"))
//!         .show(&ctx.0, |_ui| {
//!              // Terrain paint functionality goes here...
//!         });
//! }
//! ```
//! Menus are grouped by topic: `Engine` (core tools), `Render`, `Camera`, `Terrain`, and `Game` for the project's
//! own windows (see [`UIMenu`](prelude::UIMenu)).
//!
//! To see all the available UI components and how to use them, check out the [`egui`] documentation, which is the underlying UI library used by `wde_egui` and thus also by this editor plugin.
//!
//! # Default Panels
//! This crate also provides basic panels such as ones for inspecting entities and assets, and the editor's
//! [theme](prelude::ui_theme), installed on the egui context.

use bevy::prelude::*;
use wde_egui::{EguiPlugin, prelude::EguiContext};

use crate::{panels::PanelsPlugin, ui::EditorUIMenu};

mod panels;
pub mod theme;
mod ui;
mod ui_textures;

#[doc(hidden)]
pub mod prelude {
    // Re-export egui types for easier access in editor code
    pub use wde_egui::prelude::EguiContext as UIContext;

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
    pub use super::theme as ui_theme;
    pub use super::ui::{
        EditorMenuBarSet, EngineUiConfig, EngineUiSet, MenuBarVisibility, UIMenu, UIWindow
    };
    pub use super::ui_textures::{UITextureHandle, UITextures};
}

pub struct EditorPlugin;
impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((EguiPlugin, EditorUIMenu, PanelsPlugin));
        theme::install(&app.world().resource::<EguiContext>().0);
    }
}
