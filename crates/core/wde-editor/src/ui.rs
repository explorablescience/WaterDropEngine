use std::collections::{HashMap, hash_map::Entry};
use bevy::prelude::*;
use wde_egui::prelude::*;

/// The editor plugin that manages the UI menu
pub struct EditorUIMenu;
impl Plugin for EditorUIMenu {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<UIMenu>()
            .add_systems(Update, draw);
    }
}

/// Callback to draw the ui menu
fn draw(ctx: Res<EguiContext>, mut menu: ResMut<UIMenu>) {
    egui::CentralPanel::default().frame(egui::Frame::NONE).show(&ctx.0, |ui| {
        menu.draw(ui);
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
}
impl UIMenu {
    /// Add a menu item to the menu, given its path (e.g. "File/Open/Recent"). The path is split by '/' to determine the hierarchy of the menu items.
    /// If any of the intermediate items do not exist, they are created as non-leaf items. The final item is created as a leaf item.
    /// 
    /// # Arguments
    /// * `path` - The path of the menu item to add, with '/' as the separator between menu levels (e.g. "File/Open/Recent").
    /// 
    /// # Example
    /// To create a menu with a "File" menu that has an "Open" submenu, which in turn has a "Recent" submenu, you can call:
    /// ```
    /// let mut menu = UIMenu::default();
    /// menu.push("File/Open/Recent");
    /// assert!(menu.roots.contains_key("File"));
    /// 
    /// let file_menu = menu.roots.get("File").unwrap();
    /// assert!(!file_menu.leaf);
    /// assert!(file_menu.items.is_some());
    /// 
    /// let open_menu = file_menu.items.as_ref().unwrap().get("Open").unwrap();
    /// assert!(!open_menu.leaf);
    /// assert!(open_menu.items.is_some());
    /// 
    /// let recent_menu = open_menu.items.as_ref().unwrap().get("Recent").unwrap();
    /// assert!(recent_menu.leaf);
    /// assert!(recent_menu.items.is_none());
    /// ```
    /// 
    /// Then, to open for example a window when the "Recent" menu item is clicked, you can check if it is clicked using the `is_clicked` method:
    /// ```
    /// if menu.is_clicked("File/Open/Recent") {
    ///     // Open the "Recent" window
    ///     // ...
    /// }
    /// ```
    pub fn push(&mut self, path: &str) {
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
                    items: if is_leaf { None } else { Some(HashMap::new()) },
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


    /// Draw the menu using egui
    fn draw(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            let mut ordered_roots: Vec<_> = self.roots.iter().collect();
            ordered_roots.sort_by_key(|(_, item)| item.order);

            for (name, item) in ordered_roots {
                Self::draw_item(ui, item, name, &mut self.clicked);
            }
        });
    }
    fn draw_item(ui: &mut egui::Ui, item: &MenuItem, path: &str, clicked: &mut HashMap<String, bool>) {
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
