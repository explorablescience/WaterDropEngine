use bevy::prelude::*;
use std::collections::{BTreeMap, HashMap};
use wde_egui::prelude::*;

use crate::{
    panels::{FrameDataAverages, FrameDataOverlayConfig},
    theme::{self, fonts, palette},
    ui_textures::UITexturesPlugin
};

/// System set containing the menu bar's draw system, which runs in `PreUpdate` right after egui's pass begins.
///
/// Since the menu bar reserves a top strip of the screen via a [`egui::Panel`], it is drawn before every
/// `Update` system, so that windows are placed below it and a screen-filling panel (e.g. a [`egui::CentralPanel`],
/// as required by most docking crates) only gets the remaining space.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EditorMenuBarSet;

/// System set containing every system that contributes one of WaterDropEngine's own built-in
/// UI menu items (its default panels, as well as the per-plugin debug windows exposed by crates
/// such as `wde_camera` or `wde_pbr`). It is gated by [`EngineUiConfig`], so a project building
/// its own editor UI on top of `wde_editor` can add its own items to the same [`UIMenu`] without
/// the engine's own items showing up alongside them.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EngineUiSet;

/// Controls whether WaterDropEngine's own built-in UI menu items (see [`EngineUiSet`]) are
/// added. Defaults to enabled, preserving the engine's previous always-on behavior.
///
/// A consuming project that wants to hide these should insert this resource with
/// `enabled: false` *before* adding the engine's plugins: plugins only fill in the default via
/// `init_resource`, so an explicit value inserted first is left untouched.
#[derive(Resource, Clone, Copy, Debug)]
pub struct EngineUiConfig {
    pub enabled: bool
}
impl Default for EngineUiConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Whether the menu bar is shown, toggled by `key` (`None` disables the shortcut). Windows opened
/// from it stay open while it is hidden. Defaults to shown, toggled by F12.
#[derive(Resource, Clone, Copy, Debug)]
pub struct MenuBarVisibility {
    pub visible: bool,
    pub key: Option<KeyCode>
}
impl Default for MenuBarVisibility {
    fn default() -> Self {
        Self {
            visible: true,
            key: Some(KeyCode::F12)
        }
    }
}

fn toggle_menu_bar(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut visibility: ResMut<MenuBarVisibility>
) {
    if let (Some(key), Some(keys)) = (visibility.key, keys)
        && keys.just_pressed(key)
    {
        visibility.visible = !visibility.visible;
    }
}

/// The editor plugin that manages the UI menu
pub struct EditorUIMenu;
impl Plugin for EditorUIMenu {
    fn build(&self, app: &mut App) {
        app.init_resource::<EngineUiConfig>()
            .configure_sets(
                Update,
                EngineUiSet.run_if(|config: Res<EngineUiConfig>| config.enabled)
            )
            .add_plugins(UITexturesPlugin)
            .init_resource::<UIMenu>()
            .init_resource::<MenuBarVisibility>()
            .add_systems(
                PreUpdate,
                (
                    toggle_menu_bar,
                    draw.run_if(|v: Res<MenuBarVisibility>| v.visible)
                )
                    .chain()
                    .in_set(EditorMenuBarSet)
                    .after(EguiPassSet)
            );
    }
}

/// Builder of the editor's windows: an [`egui::Window`] whose content scrolls instead of being cut when it is
/// taller than the screen. Use [`UIWindow::with`] for the window options that are not forwarded here.
pub struct UIWindow<'open>(egui::Window<'open>);
impl<'open> UIWindow<'open> {
    pub fn new(title: impl Into<egui::WidgetText>) -> Self {
        Self(egui::Window::new(title))
    }

    /// Applies any [`egui::Window`] option.
    pub fn with(self, f: impl FnOnce(egui::Window<'open>) -> egui::Window<'open>) -> Self {
        Self(f(self.0))
    }
    pub fn open(self, open: &'open mut bool) -> Self {
        Self(self.0.open(open))
    }
    pub fn resizable(self, resizable: bool) -> Self {
        Self(self.0.resizable(resizable))
    }
    pub fn collapsible(self, collapsible: bool) -> Self {
        Self(self.0.collapsible(collapsible))
    }
    pub fn default_pos(self, pos: impl Into<egui::Pos2>) -> Self {
        Self(self.0.default_pos(pos))
    }
    pub fn default_width(self, width: f32) -> Self {
        Self(self.0.default_width(width))
    }
    pub fn default_size(self, size: impl Into<egui::Vec2>) -> Self {
        Self(self.0.default_size(size))
    }

    pub fn show<R>(
        self,
        ctx: &egui::Context,
        add_contents: impl FnOnce(&mut egui::Ui) -> R
    ) -> Option<egui::InnerResponse<Option<R>>> {
        // `Window::vscroll` would cap the content at the window's default height, so the scroll area gets its own
        // rect, as tall as the space below the menu bar minus the title bar
        let style = ctx.global_style();
        let title_height = style.spacing.interact_size.y.max(fonts::SIZE_HEADING * 1.5);
        let max_height = (ctx.root_ui(|ui| ui.available_rect_before_wrap()).height()
            - title_height
            - 2.0 * style.spacing.window_margin.sum().y
            - 2.0 * style.spacing.item_spacing.y)
            .max(100.0);
        self.0.show(ctx, |ui| {
            let rect = egui::Rect::from_min_size(
                ui.cursor().min,
                egui::vec2(ui.available_width(), max_height)
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                egui::ScrollArea::vertical()
                    .max_height(max_height)
                    .show(ui, add_contents)
                    .inner
            })
            .inner
        })
    }
}

fn draw(
    ctx: Res<EguiContext>,
    mut menu: ResMut<UIMenu>,
    stats_config: Res<FrameDataOverlayConfig>,
    stats: Res<FrameDataAverages>
) {
    let style = menu.style.clone().unwrap_or_else(theme::menu_style);
    let frame = egui::Frame::NONE
        .fill(style.visuals.extreme_bg_color)
        .inner_margin(egui::Margin::symmetric(8, 3));

    ctx.0.root_ui(|root| {
        egui::Panel::top("wde_editor_menu_bar")
            .frame(frame)
            .show(root, |ui| {
                let config = egui::containers::menu::MenuConfig::new().style(style.clone());
                egui::MenuBar::new()
                    .style(style)
                    .config(config)
                    .ui(ui, |ui| {
                        menu.draw_bar(ui);
                        if stats_config.enabled {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    frame_stats(ui, &stats);
                                }
                            );
                        }
                    });
            });
    });
}

fn frame_stats(ui: &mut egui::Ui, stats: &FrameDataAverages) {
    // Laid out from the right
    let stat = |ui: &mut egui::Ui, value: Option<String>, unit: &str| {
        let font = theme::body_font(fonts::SIZE_SMALL);
        ui.label(
            egui::RichText::new(unit)
                .font(font.clone())
                .color(palette::TEXT_DISABLED)
        );
        ui.add_space(-4.0);
        ui.label(
            egui::RichText::new(value.unwrap_or_else(|| "-".into()))
                .font(font)
                .color(palette::TEXT_MUTED)
        );
    };
    ui.spacing_mut().item_spacing.x = 8.0;
    stat(ui, stats.frame_ms.map(|v| format!("{v:.1}")), "ms");
    stat(ui, stats.fps.map(|v| format!("{v:.0}")), "FPS");
}

/// Root menus and sections created up front, so that they keep this order whatever the order in which plugins
/// add their items. Other roots and sections follow in alphabetical order.
const LAYOUT: &[&str] = &[
    "File",
    "Engine",
    "Render",
    "Render/Pipeline",
    "Camera",
    "Terrain",
    "Game"
];

/// The children of a menu or section, by title.
type Branch = BTreeMap<String, MenuNode>;

#[derive(Clone, Debug, Default)]
struct MenuNode {
    /// Index in [`LAYOUT`], `usize::MAX` for the other nodes.
    rank: usize,
    /// The children by title, `None` for an item.
    children: Option<Branch>
}
impl MenuNode {
    fn branch(rank: usize) -> Self {
        Self {
            rank,
            children: Some(BTreeMap::new())
        }
    }

    /// The items in alphabetical order, then the non-empty branches in layout order.
    fn sorted(children: &Branch) -> (Vec<&String>, Vec<(&String, &Branch)>) {
        let items = children
            .iter()
            .filter(|(_, node)| node.children.is_none())
            .map(|(name, _)| name)
            .collect();
        let mut branches: Vec<_> = children
            .iter()
            .filter_map(|(name, node)| Some((name, node.rank, node.children.as_ref()?)))
            .filter(|(_, _, children)| !children.is_empty())
            .collect();
        branches.sort_by_key(|(_, rank, _)| *rank);
        let branches = branches
            .into_iter()
            .map(|(name, _, children)| (name, children))
            .collect();
        (items, branches)
    }
}

/// The menu bar at the top of the screen, whose items toggle a state, usually whether a window is open.
///
/// An item is added the first time its state is requested with its path, e.g. `"Render/Shadows"`:
/// - the first part is the menu in the bar. Menus are drawn in the order of [`LAYOUT`] (`File`, `Engine`,
///   `Render`, `Camera`, `Terrain`, `Game`), then alphabetically;
/// - a 3-part path such as `"Render/Pipeline/Passes"` adds the item to a titled section of the menu, drawn after
///   the menu's own items. A deeper path opens a submenu inside that section.
///
/// Items are sorted alphabetically, and those that are on are marked with a dot.
#[derive(Resource, Debug)]
pub struct UIMenu {
    roots: Branch,
    /// The state of each item, by path.
    clicked: HashMap<String, bool>,

    /// Style of the menu bar, [`theme::menu_style`] when unset.
    style: Option<egui::Style>,
    /// Color of the "WDE" title drawn on the left side of the menu bar, the theme accent when unset.
    title_color: Option<egui::Color32>
}
impl Default for UIMenu {
    fn default() -> Self {
        let mut menu = Self {
            roots: BTreeMap::new(),
            clicked: HashMap::new(),
            style: None,
            title_color: None
        };
        for (rank, path) in LAYOUT.iter().enumerate() {
            let parts: Vec<&str> = path.split('/').collect();
            let (last, parents) = parts.split_last().unwrap();
            Self::branch_mut(&mut menu.roots, parents)
                .entry(last.to_string())
                .or_insert_with(|| MenuNode::branch(rank));
        }
        menu
    }
}
impl UIMenu {
    /// Set the style of the menu bar and its menus. Pass `None` for the default [`theme::menu_style`].
    pub fn set_style(&mut self, style: Option<egui::Style>) {
        self.style = style;
    }
    pub fn style(&self) -> Option<&egui::Style> {
        self.style.as_ref()
    }

    /// Set the color of the "WDE" title drawn on the left side of the menu bar. Pass `None` for the theme accent.
    pub fn set_title_color(&mut self, color: Option<egui::Color32>) {
        self.title_color = color;
    }

    /// Returns the children of the branch at `path`, creating the missing branches.
    fn branch_mut<'a>(mut items: &'a mut Branch, path: &[&str]) -> &'a mut Branch {
        for name in path {
            let node = items
                .entry(name.to_string())
                .or_insert_with(|| MenuNode::branch(usize::MAX));
            items = node.children.get_or_insert_default();
        }
        items
    }

    /// Adds the item at `path`, e.g. `"Render/Pipeline/Passes"`. A branch at the same path is kept as is.
    fn push(&mut self, path: &str) {
        let parts: Vec<&str> = path.split('/').collect();
        let (last, parents) = parts.split_last().unwrap();
        Self::branch_mut(&mut self.roots, parents)
            .entry(last.to_string())
            .or_insert_with(MenuNode::default);
    }

    /// Return true if the item at the given path is on. If the item does not exist, return false.
    pub fn is_clicked(&self, path: &str) -> bool {
        self.clicked.get(path).cloned().unwrap_or(false)
    }
    /// Returns a mutable reference to the state of the item at the given path.
    /// If the item does not exist, it is created and its state is initialized to false.
    pub fn clicked_mut(&mut self, path: &str) -> &mut bool {
        self.clicked_mut_or(path, false)
    }
    /// Returns a mutable reference to the state of the item at the given path.
    /// If the item does not exist, it is created and its state is initialized to true.
    pub fn clicked_mut_default_open(&mut self, path: &str) -> &mut bool {
        self.clicked_mut_or(path, true)
    }
    fn clicked_mut_or(&mut self, path: &str, default: bool) -> &mut bool {
        if !self.clicked.contains_key(path) {
            self.push(path);
            self.clicked.insert(path.to_string(), default);
        }
        self.clicked.get_mut(path).unwrap()
    }

    fn draw_bar(&mut self, ui: &mut egui::Ui) {
        let title_color = self.title_color.unwrap_or(palette::ACCENT);
        ui.label(
            egui::RichText::new("WDE")
                .font(theme::heading_font(13.0))
                .color(title_color)
        );
        ui.add_space(8.0);

        let (items, branches) = MenuNode::sorted(&self.roots);
        for name in items {
            let open = self.clicked.entry(name.clone()).or_default();
            if ui.add(egui::Button::selectable(*open, name)).clicked() {
                *open = !*open;
            }
        }
        for (name, children) in branches {
            ui.menu_button(name, |ui| {
                Self::draw_menu(ui, children, name, &mut self.clicked);
            });
        }
    }

    fn draw_menu(
        ui: &mut egui::Ui,
        children: &Branch,
        path: &str,
        clicked: &mut HashMap<String, bool>
    ) {
        let (items, sections) = MenuNode::sorted(children);
        let has_items = !items.is_empty();
        for name in items {
            Self::draw_item(
                ui,
                name,
                clicked.entry(format!("{path}/{name}")).or_default()
            );
        }

        for (i, (name, section)) in sections.into_iter().enumerate() {
            if has_items || i > 0 {
                ui.separator();
            }
            egui::Frame::NONE
                .inner_margin(egui::Margin {
                    left: ui.spacing().button_padding.x as i8,
                    right: 0,
                    top: 2,
                    bottom: 1
                })
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(name.to_uppercase())
                                .font(theme::heading_font(fonts::SIZE_SMALL))
                                .color(palette::TEXT_DISABLED)
                        )
                        .selectable(false)
                    );
                });

            let section_path = format!("{path}/{name}");
            let (items, submenus) = MenuNode::sorted(section);
            for name in items {
                let open = clicked.entry(format!("{section_path}/{name}")).or_default();
                Self::draw_item(ui, name, open);
            }
            for (name, children) in submenus {
                let submenu_path = format!("{section_path}/{name}");
                ui.menu_button(name, |ui| {
                    Self::draw_menu(ui, children, &submenu_path, clicked);
                });
            }
        }
    }

    fn draw_item(ui: &mut egui::Ui, title: &str, open: &mut bool) {
        let (text_color, mark_color) = match *open {
            true => (palette::TEXT, palette::ACCENT),
            false => (palette::TEXT_MUTED, egui::Color32::TRANSPARENT)
        };
        let button = egui::Button::new(egui::RichText::new(title).color(text_color))
            .right_text(egui::RichText::new("●").size(8.0).color(mark_color))
            .min_size(egui::vec2(150.0, 0.0));
        if ui.add(button).clicked() {
            *open = !*open;
        }
    }
}
