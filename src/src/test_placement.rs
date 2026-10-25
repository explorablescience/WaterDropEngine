//! Build menu at the bottom of the screen, built with `wde_ui`: a bar of building categories that
//! expand into building cards, and the move / remove tools.
use bevy::prelude::*;
use wde::prelude::{Color as WdeColor, *};

pub struct BuildMenuPlugin;
impl Plugin for BuildMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BuildMenu>()
            .add_systems(Startup, spawn_build_menu)
            .add_systems(
                Update,
                (
                    rebuild_build_menu,
                    handle_clicks,
                    handle_escape,
                    update_highlights,
                    update_info_panel
                )
                    .chain()
            );
    }
}

const PANEL: WdeColor = WdeColor::Srgba(0.07, 0.08, 0.1, 0.9);
const BUTTON: WdeColor = WdeColor::Srgba(0.14, 0.16, 0.19, 1.0);
const BUTTON_HOVERED: WdeColor = WdeColor::Srgba(0.21, 0.24, 0.29, 1.0);
const BUTTON_PRESSED: WdeColor = WdeColor::Srgba(0.11, 0.25, 0.33, 1.0);
const OUTLINE: WdeColor = WdeColor::Srgba(0.25, 0.28, 0.33, 1.0);
const ACCENT: WdeColor = WdeColor::Srgba(0.35, 0.8, 1.0, 1.0);
const MUTED: WdeColor = WdeColor::Srgba(0.6, 0.64, 0.7, 1.0);

/// The category whose building cards are shown.
#[derive(Resource, Default)]
struct BuildMenu {
    open: Option<String>
}

/// Containers refilled when the placement config or the open category changes.
#[derive(Resource)]
struct BuildMenuNodes {
    categories: Entity,
    tray: Entity
}

#[derive(Component)]
struct CategoryButton(String);

/// A building card, with what the info panel shows when it is hovered.
#[derive(Component)]
struct BuildCard {
    tool: PlacementTool,
    title: String,
    meta: String,
    description: String
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum ToolButton {
    Move,
    Remove,
    Cutaway
}

#[derive(Component, Clone, Copy)]
enum InfoText {
    Title,
    Meta,
    Description,
    Hint
}

#[derive(Component)]
struct InfoPanel;

fn panel(style: UiStyle) -> impl Bundle {
    (
        UiNode(UiStyle {
            padding: UiRect::all(Val::Px(8.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..style
        }),
        UiBackground(PANEL),
        UiBorderColor(OUTLINE),
        UiCornerRadius(12.0)
    )
}

/// A clickable square, highlighted by `update_highlights`.
fn button(width: f32, height: f32) -> impl Bundle {
    (
        UiNode(UiStyle {
            width: Val::Px(width),
            height: Val::Px(height),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::CENTER,
            justify_content: JustifyContent::CENTER,
            row_gap: Val::Px(4.0),
            padding: UiRect::all(Val::Px(4.0)),
            border: UiRect::all(Val::Px(2.0)),
            flex_shrink: 0.0,
            ..default()
        }),
        UiBackground(BUTTON),
        UiBorderColor(BUTTON),
        UiCornerRadius(8.0),
        UiInteraction::default()
    )
}

/// An icon, or an empty square of the same size if the config has none.
fn spawn_icon(parent: &mut ChildSpawnerCommands, texture: Option<&Handle<Texture>>, size: f32) {
    let mut icon = parent.spawn(UiNode(UiStyle {
        width: Val::Px(size),
        height: Val::Px(size),
        flex_shrink: 0.0,
        ..default()
    }));
    if let Some(texture) = texture {
        icon.insert(UiImage::new(texture.clone()));
    }
}

fn spawn_build_menu(mut commands: Commands) {
    let mut nodes = None;
    commands
        .spawn((
            Name::new("Build Menu"),
            UiNode(UiStyle {
                position: UiPosition::Absolute,
                inset: UiRect {
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    bottom: Val::Px(12.0),
                    top: Val::Auto
                },
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::CENTER,
                row_gap: Val::Px(8.0),
                ..default()
            })
        ))
        .with_children(|menu| {
            menu.spawn((
                InfoPanel,
                panel(UiStyle {
                    display: UiDisplay::None,
                    width: Val::Px(380.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    ..default()
                })
            ))
            .with_children(|info| {
                info.spawn((InfoText::Title, UiText::new("").with_size(16.0).bold()));
                info.spawn((
                    InfoText::Meta,
                    UiText::new("").with_size(12.0).with_color(ACCENT)
                ));
                info.spawn((InfoText::Description, UiText::new("").with_size(13.0)));
                info.spawn((
                    InfoText::Hint,
                    UiText::new("").with_size(12.0).with_color(MUTED)
                ));
            });

            let tray = menu
                .spawn(panel(UiStyle {
                    display: UiDisplay::None,
                    column_gap: Val::Px(6.0),
                    ..default()
                }))
                .id();

            let mut categories = Entity::PLACEHOLDER;
            menu.spawn(panel(UiStyle {
                align_items: AlignItems::CENTER,
                column_gap: Val::Px(6.0),
                ..default()
            }))
            .with_children(|bar| {
                categories = bar
                    .spawn(UiNode(UiStyle {
                        column_gap: Val::Px(6.0),
                        ..default()
                    }))
                    .id();
                bar.spawn((
                    UiNode(UiStyle {
                        width: Val::Px(1.0),
                        height: Val::Px(56.0),
                        margin: UiRect::axes(Val::Px(6.0), Val::Px(0.0)),
                        ..default()
                    }),
                    UiBackground(OUTLINE)
                ));
                for (tool, label) in [
                    (ToolButton::Move, "Move"),
                    (ToolButton::Remove, "Remove"),
                    (ToolButton::Cutaway, "Cutaway")
                ] {
                    bar.spawn((tool, button(72.0, 72.0)))
                        .with_child(UiText::new(label).with_size(12.0).bold());
                }
            });
            nodes = Some(BuildMenuNodes { categories, tray });
        });
    commands.insert_resource(nodes.unwrap());
}

/// Network pieces are drawn with the line tool: one card per network, from its straight piece.
fn is_network_card(entry: &PlacementConfigEntry) -> bool {
    entry
        .network
        .as_ref()
        .is_some_and(|network| !network.junction)
}

fn rebuild_build_menu(
    mut commands: Commands,
    config: Res<PlacementConfig>,
    menu: Res<BuildMenu>,
    nodes: Res<BuildMenuNodes>,
    mut tray_node: Query<&mut UiNode>,
    mut built: Local<Option<(usize, Option<String>)>>
) {
    // Entries are added as their models load
    let state = (config.entries.len(), menu.open.clone());
    if built.as_ref() == Some(&state) {
        return;
    }
    *built = Some(state);

    commands.entity(nodes.categories).despawn_children();
    for category in &config.categories {
        let first = config.entries.iter().find(|e| &e.category == category);
        commands.entity(nodes.categories).with_children(|bar| {
            bar.spawn((CategoryButton(category.clone()), button(84.0, 72.0)))
                .with_children(|button| {
                    spawn_icon(button, first.and_then(|e| e.icon.as_ref()), 40.0);
                    button.spawn(UiText::new(category.clone()).with_size(11.0));
                });
        });
    }

    commands.entity(nodes.tray).despawn_children();
    let entries: Vec<&PlacementConfigEntry> = config
        .entries
        .iter()
        .filter(|e| Some(&e.category) == menu.open.as_ref())
        .filter(|e| e.network.is_none() || is_network_card(e))
        .collect();
    if let Ok(mut tray) = tray_node.get_mut(nodes.tray) {
        tray.0.display = match entries.is_empty() {
            true => UiDisplay::None,
            false => UiDisplay::Flex
        };
    }
    for entry in entries {
        let card = match &entry.network {
            Some(network) => BuildCard {
                tool: PlacementTool::DrawNetwork(network.name.clone()),
                title: format!("{}s", capitalize(&network.name)),
                meta: "Network".to_string(),
                description: format!(
                    "Drag on the terrain to lay {0}s. Straights, corners and junctions are \
                     chosen automatically, and existing {0}s are joined.",
                    network.name
                )
            },
            None => BuildCard {
                tool: PlacementTool::Build(entry.label.clone()),
                title: entry.label.clone(),
                meta: match entry.module {
                    true => format!("{}×{} module", entry.extent.x, entry.extent.y),
                    false => format!("{}×{} tiles", entry.extent.x, entry.extent.y)
                },
                description: entry.description.clone()
            }
        };
        let title = card.title.clone();
        commands.entity(nodes.tray).with_children(|tray| {
            tray.spawn((card, button(96.0, 112.0)))
                .with_children(|card| {
                    spawn_icon(card, entry.icon.as_ref(), 64.0);
                    card.spawn(UiText::new(title).with_size(11.0));
                });
        });
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

fn handle_clicks(
    mut clicks: MessageReader<UiClick>,
    mut menu: ResMut<BuildMenu>,
    tool: Res<PlacementTool>,
    mut cutaway: ResMut<BuildingCutaway>,
    mut placement: MessageWriter<PlacementCommand>,
    (categories, cards, tools): (
        Query<&CategoryButton>,
        Query<&BuildCard>,
        Query<&ToolButton>
    )
) {
    for click in clicks.read() {
        if let Ok(CategoryButton(category)) = categories.get(click.entity) {
            menu.open = match menu.open.as_ref() == Some(category) {
                true => None,
                false => Some(category.clone())
            };
        } else if let Ok(card) = cards.get(click.entity) {
            let command = match (&card.tool, *tool == card.tool) {
                (_, true) => PlacementCommand::Cancel,
                (PlacementTool::Build(label), _) => PlacementCommand::Build(label.clone()),
                (PlacementTool::DrawNetwork(network), _) => {
                    PlacementCommand::DrawNetwork(network.clone())
                }
                _ => continue
            };
            placement.write(command);
        } else if let Ok(button) = tools.get(click.entity) {
            match button {
                ToolButton::Move if *tool == PlacementTool::Move => {
                    placement.write(PlacementCommand::Cancel);
                }
                ToolButton::Move => {
                    placement.write(PlacementCommand::Move);
                }
                ToolButton::Remove if *tool == PlacementTool::Remove => {
                    placement.write(PlacementCommand::Cancel);
                }
                ToolButton::Remove => {
                    placement.write(PlacementCommand::Remove);
                }
                ToolButton::Cutaway => cutaway.0 = !cutaway.0
            }
        }
    }
}

/// Escape drops the active tool, then closes the category.
fn handle_escape(
    keys: Res<ButtonInput<KeyCode>>,
    tool: Res<PlacementTool>,
    mut menu: ResMut<BuildMenu>,
    mut placement: MessageWriter<PlacementCommand>
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    if *tool != PlacementTool::None {
        placement.write(PlacementCommand::Cancel);
    } else if menu.open.is_some() {
        menu.open = None;
    }
}

fn update_highlights(
    menu: Res<BuildMenu>,
    tool: Res<PlacementTool>,
    cutaway: Res<BuildingCutaway>,
    mut buttons: Query<(
        &UiInteraction,
        &mut UiBackground,
        &mut UiBorderColor,
        AnyOf<(&CategoryButton, &BuildCard, &ToolButton)>
    )>
) {
    for (interaction, mut background, mut border, (category, card, tool_button)) in &mut buttons {
        let selected = category.is_some_and(|c| menu.open.as_ref() == Some(&c.0))
            || card.is_some_and(|c| c.tool == *tool)
            || tool_button.is_some_and(|button| match button {
                ToolButton::Move => *tool == PlacementTool::Move,
                ToolButton::Remove => *tool == PlacementTool::Remove,
                ToolButton::Cutaway => cutaway.0
            });
        let fill = match interaction {
            UiInteraction::Pressed => BUTTON_PRESSED,
            UiInteraction::Hovered => BUTTON_HOVERED,
            UiInteraction::None => BUTTON
        };
        let outline = if selected { ACCENT } else { fill };
        if background.0 != fill {
            background.0 = fill;
        }
        if border.0 != outline {
            border.0 = outline;
        }
    }
}

/// Describes the hovered card, or how to use the active tool.
fn update_info_panel(
    pointer: Res<UiPointer>,
    tool: Res<PlacementTool>,
    cards: Query<&BuildCard>,
    mut panel: Single<&mut UiNode, With<InfoPanel>>,
    mut texts: Query<(&InfoText, &mut UiText, &mut UiNode), Without<InfoPanel>>
) {
    let hovered = pointer.hovered.and_then(|entity| cards.get(entity).ok());
    let active = cards.iter().find(|card| card.tool == *tool);
    let hint = match &*tool {
        PlacementTool::Build(_) => "Left click: place   •   Middle click: rotate   •   Esc: cancel",
        PlacementTool::DrawNetwork(_) => {
            "Drag: draw a line   •   Right click: cancel the line   •   Esc: stop"
        }
        PlacementTool::Move => "Click a placed building to pick it up again   •   Esc: stop",
        PlacementTool::Remove => "Click a placed building to demolish it   •   Esc: stop",
        PlacementTool::None => "Click to build"
    };
    // The active tool's card is not shown while another category is open
    let (title, meta, description) = match (hovered.or(active), &*tool) {
        (Some(card), _) => (
            card.title.clone(),
            card.meta.as_str(),
            card.description.as_str()
        ),
        (None, PlacementTool::Build(label)) => (label.clone(), "Building", ""),
        (None, PlacementTool::DrawNetwork(network)) => {
            (format!("{}s", capitalize(network)), "Network", "")
        }
        (None, PlacementTool::Move) => ("Move".to_string(), "Tool", ""),
        (None, PlacementTool::Remove) => ("Remove".to_string(), "Tool", ""),
        (None, PlacementTool::None) => {
            if panel.0.display != UiDisplay::None {
                panel.0.display = UiDisplay::None;
            }
            return;
        }
    };
    if panel.0.display == UiDisplay::None {
        panel.0.display = UiDisplay::Flex;
    }
    for (field, mut text, mut node) in &mut texts {
        let value = match field {
            InfoText::Title => title.as_str(),
            InfoText::Meta => meta,
            InfoText::Description => description,
            InfoText::Hint => hint
        };
        if text.text != value {
            text.text = value.to_string();
        }
        // An empty line would still take a row gap
        let display = match value.is_empty() {
            true => UiDisplay::None,
            false => UiDisplay::Flex
        };
        if node.0.display != display {
            node.0.display = display;
        }
    }
}
