use bevy::prelude::*;
use puffin_egui::egui::{RichText, Stroke};
use std::collections::{HashMap, hash_map::Entry};
use wde_egui::prelude::*;

use crate::ui_textures::UITexturesPlugin;

/// System set containing the menu bar's draw system.
///
/// Since the menu bar reserves a top strip of the screen via a [`egui::TopBottomPanel`],
/// any other plugin that draws a screen-filling panel (e.g. a [`egui::CentralPanel`],
/// as required by most docking crates) must run its own drawing system after this set,
/// otherwise both panels will compute their layout before the other has claimed its space.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EditorMenuBarSet;

/// The editor plugin that manages the UI menu
pub struct EditorUIMenu;
impl Plugin for EditorUIMenu {
    fn build(&self, app: &mut App) {
        app.add_plugins(UITexturesPlugin)
            .init_resource::<UIMenu>()
            .add_systems(Update, draw.in_set(EditorMenuBarSet));
    }
}

/// Callback to draw the ui menu
fn draw(ctx: Res<EguiContext>, mut menu: ResMut<UIMenu>) {
    let style = ctx.0.style();
    let frame = egui::Frame::NONE
        .fill(style.visuals.extreme_bg_color)
        .stroke(Stroke::NONE)
        .inner_margin(egui::Margin {
            left: 8,
            right: 8,
            top: 12,
            bottom: 16
        });

    egui::TopBottomPanel::top("wde_editor_menu_bar")
        .frame(frame)
        .show(&ctx.0, |ui| {
            let style = menu.style.clone();
            menu.draw(ui, style);
        });
}

/// Store the menu structure of a UI menu.
/// Note that this item is not necessarily the root of the menu, it can be a submenu as well.
///
/// To add a menu item to the menu, use the `push` method of `UIMenu`, which takes care of creating the intermediate items if necessary.
#[derive(Clone, Debug)]
struct MenuItem {
    /// The title of the menu item
    title: String,
    /// Insertion order within its menu level.
    order: usize,
    /// Whether this menu item is a leaf (i.e. it has no subitems)
    leaf: bool,
    /// The list of subitems, if this item is not a leaf. None if this item is a leaf.
    items: Option<HashMap<String, MenuItem>>
}

#[derive(Resource, Default, Debug)]
pub struct UIMenu {
    /// The list of root menu items along with their name. Each item can have subitems, and so on.
    roots: HashMap<String, MenuItem>,
    /// The list of menu items (only leafs) that are currently clicked (i.e. their subitems are visible).
    /// The key is the path of the menu item (e.g. "File/Open/Recent").
    clicked: HashMap<String, bool>,
    /// Monotonic counter used to assign insertion order to newly created menu items.
    next_order: usize,

    /// Style of the menu bar, used to draw the menu bar with a custom style.
    style: Option<egui::Style>
}
impl UIMenu {
    /// Set the style of the menu bar, used to draw the menu bar with a custom style.
    pub fn set_style(&mut self, style: Option<egui::Style>) {
        self.style = style;
    }

    /// Add a menu item to the menu, given its path (e.g. "File/Open/Recent"). The path is split by '/' to determine the hierarchy of the menu items.
    /// If any of the intermediate items do not exist, they are created as non-leaf items. The final item is created as a leaf item.
    fn push(&mut self, path: &str) {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.is_empty() {
            return;
        }

        Self::push_item(&mut self.roots, &parts, &mut self.next_order);
    }
    fn push_item(items: &mut HashMap<String, MenuItem>, path: &[&str], next_order: &mut usize) {
        if path.is_empty() {
            return;
        }

        let name = path[0];
        let is_leaf = path.len() == 1;

        let item = match items.entry(name.to_string()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let order = *next_order;
                *next_order += 1;
                entry.insert(MenuItem {
                    title: name.to_string(),
                    order,
                    leaf: is_leaf,
                    items: if is_leaf { None } else { Some(HashMap::new()) }
                })
            }
        };

        if !is_leaf && item.items.is_none() {
            item.leaf = false;
            item.items = Some(HashMap::new());
        }

        if !is_leaf {
            Self::push_item(item.items.as_mut().unwrap(), &path[1..], next_order);
        }
    }

    /// Return true if the menu item at the given path is clicked (i.e. its subitems are visible). If the item does not exist or is a leaf, return false.
    pub fn is_clicked(&self, path: &str) -> bool {
        self.clicked.get(path).cloned().unwrap_or(false)
    }
    /// Returns a mutable reference to the clicked state of the menu item at the given path
    /// If the item does not exist, it is created as a leaf item and its clicked state is initialized to false.
    pub fn clicked_mut(&mut self, path: &str) -> &mut bool {
        if !self.clicked.contains_key(path) {
            self.push(path);
            self.clicked.insert(path.to_string(), false);
        }
        self.clicked.get_mut(path).unwrap()
    }

    /// Draw the menu using egui
    fn draw(&mut self, ui: &mut egui::Ui, style: Option<egui::Style>) {
        // Draw a background for the menu bar
        let visuals = style
            .as_ref()
            .map(|s| &s.visuals)
            .unwrap_or(&ui.style().visuals);
        let bg_color = visuals.extreme_bg_color;
        let rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(rect, 0.0, bg_color);

        // Draw the menu bar
        ui.scope(|ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                let button_padding = style
                    .as_ref()
                    .map(|s| s.spacing.button_padding)
                    .unwrap_or(ui.style().spacing.button_padding);

                if let Some(style) = style {
                    ui.set_style(style);
                }

                // Write "WDE" on the left side of the menu bar
                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(button_padding.x as i8, button_padding.y as i8))
                    .show(ui, |ui| {
                        ui.label(RichText::new("WDE").heading().strong());
                    });
                ui.add_space(14.0);

                // Draw the menu items in order of their insertion
                let mut ordered_roots: Vec<_> = self.roots.iter().collect();
                ordered_roots.sort_by_key(|(_, item)| item.order);
                for (name, item) in ordered_roots {
                    Self::draw_item(ui, item, name, &mut self.clicked);
                }
            });
        });
    }
    fn draw_item(
        ui: &mut egui::Ui,
        item: &MenuItem,
        path: &str,
        clicked: &mut HashMap<String, bool>
    ) {
        if item.leaf {
            if ui.button(&item.title).clicked() {
                // Toggle the clicked state of the leaf item
                let is_clicked = clicked.get(path).cloned().unwrap_or(false);
                clicked.insert(path.to_string(), !is_clicked);
            }
        } else {
            ui.menu_button(&item.title, |ui| {
                let mut ordered_items: Vec<_> = item.items.as_ref().unwrap().iter().collect();
                ordered_items.sort_by_key(|(_, subitem)| subitem.order);

                for (name, subitem) in ordered_items {
                    let subpath = format!("{}/{}", path, name);
                    Self::draw_item(ui, subitem, &subpath, clicked);
                }
            });
        }
    }
}
