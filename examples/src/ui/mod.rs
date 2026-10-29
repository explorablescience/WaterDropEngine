//! Game UI demo: a strategy game HUD laid out with `wde_ui`, on top of the scene.
//!
//! - The top bar holds resource chips and an animated energy gauge (relaid out every frame).
//! - The left panel lists units, rows highlight on hover.
//! - The minimap places markers with absolute positioning.
//! - The first toolbar button toggles the units panel, the others log their click.
use bevy::prelude::*;
use wde::prelude::{WdeColor, *};

pub struct UiDemoPlugin;
impl Plugin for UiDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (init_scene, init_ui)).add_systems(
            Update,
            (update_button_colors, handle_toolbar_clicks, animate_energy)
        );
    }
}

const PANEL: WdeColor = WdeColor::Srgba(0.07, 0.08, 0.1, 0.88);
const PANEL_INSET: WdeColor = WdeColor::Srgba(0.13, 0.15, 0.18, 1.0);
const OUTLINE: WdeColor = WdeColor::Srgba(0.25, 0.28, 0.33, 1.0);
const PLACEHOLDER: WdeColor = WdeColor::Srgba(0.55, 0.6, 0.66, 1.0);
const RESOURCES: [WdeColor; 4] = [
    WdeColor::Srgba(0.35, 0.8, 1.0, 1.0),
    WdeColor::Srgba(1.0, 0.78, 0.25, 1.0),
    WdeColor::Srgba(0.45, 0.9, 0.45, 1.0),
    WdeColor::Srgba(0.95, 0.4, 0.35, 1.0)
];

/// Background colors of an interactive node, per [`UiInteraction`] state.
#[derive(Component, Clone, Copy)]
struct ButtonColors {
    idle: WdeColor,
    hovered: WdeColor,
    pressed: WdeColor
}
impl ButtonColors {
    const TOOLBAR: Self = Self {
        idle: WdeColor::Srgba(0.16, 0.18, 0.22, 1.0),
        hovered: WdeColor::Srgba(0.24, 0.28, 0.34, 1.0),
        pressed: WdeColor::Srgba(0.35, 0.8, 1.0, 1.0)
    };
    const ROW: Self = Self {
        idle: WdeColor::Srgba(0.0, 0.0, 0.0, 0.0),
        hovered: WdeColor::Srgba(1.0, 1.0, 1.0, 0.08),
        pressed: WdeColor::Srgba(1.0, 1.0, 1.0, 0.16)
    };

    fn bundle(self) -> impl Bundle {
        (self, UiBackground(self.idle), UiInteraction::default())
    }
}

#[derive(Component)]
struct ToolbarButton(usize);

#[derive(Component)]
struct UnitsPanel;

#[derive(Component)]
struct EnergyLevel;

fn init_scene(mut commands: Commands) {
    commands.spawn((
        Name::new("Main Camera"),
        ActiveCamera,
        Transform::from_xyz(0.0, 6.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y)
    ));
}

fn panel(style: UiStyle) -> impl Bundle {
    (
        UiNode(UiStyle {
            padding: UiRect::all(Val::Px(10.0)),
            ..style
        }),
        UiBackground(PANEL),
        UiBorderColor(OUTLINE),
        UiCornerRadius(10.0)
    )
}

fn rect(width: f32, height: f32, color: WdeColor, radius: f32) -> impl Bundle {
    (
        UiNode(UiStyle {
            width: Val::Px(width),
            height: Val::Px(height),
            flex_shrink: 0.0,
            ..default()
        }),
        UiBackground(color),
        UiCornerRadius(radius)
    )
}

fn init_ui(mut commands: Commands) {
    // Full-window root: no background, so the pointer passes through to the game
    commands
        .spawn((
            Name::new("HUD"),
            UiNode(UiStyle {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SPACE_BETWEEN,
                // The top one clears the editor menu bar
                padding: UiRect::px(12.0, 12.0, 60.0, 12.0),
                ..default()
            })
        ))
        .with_children(|hud| {
            spawn_top_bar(hud);
            hud.spawn(UiNode(UiStyle {
                flex_grow: 1.0,
                justify_content: JustifyContent::SPACE_BETWEEN,
                align_items: AlignItems::FLEX_START,
                padding: UiRect::axes(Val::Px(0.0), Val::Px(12.0)),
                ..default()
            }))
            .with_children(|middle| {
                spawn_units_panel(middle);
                spawn_minimap(middle);
            });
            spawn_toolbar(hud);
        });
}

fn spawn_top_bar(hud: &mut ChildSpawnerCommands) {
    hud.spawn(panel(UiStyle {
        height: Val::Px(48.0),
        align_items: AlignItems::CENTER,
        column_gap: Val::Px(10.0),
        border: UiRect::all(Val::Px(1.0)),
        ..default()
    }))
    .with_children(|bar| {
        for color in RESOURCES {
            bar.spawn((
                UiNode(UiStyle {
                    width: Val::Px(110.0),
                    height: Val::Px(28.0),
                    align_items: AlignItems::CENTER,
                    column_gap: Val::Px(8.0),
                    padding: UiRect::axes(Val::Px(6.0), Val::Px(0.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                }),
                UiBackground(PANEL_INSET),
                UiBorderColor(color),
                UiCornerRadius(14.0)
            ))
            .with_children(|chip| {
                chip.spawn(rect(16.0, 16.0, color, 8.0));
                chip.spawn(rect(48.0, 6.0, PLACEHOLDER, 3.0));
            });
        }

        bar.spawn(UiNode(UiStyle {
            flex_grow: 1.0,
            ..default()
        }));

        // Energy gauge, its fill is animated by `animate_energy`
        bar.spawn((
            UiNode(UiStyle {
                width: Val::Px(220.0),
                height: Val::Px(14.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            }),
            UiBackground(PANEL_INSET),
            UiBorderColor(OUTLINE),
            UiCornerRadius(7.0)
        ))
        .with_child((
            EnergyLevel,
            UiNode(UiStyle {
                width: Val::Percent(70.0),
                height: Val::Percent(100.0),
                ..default()
            }),
            UiBackground(RESOURCES[0]),
            UiCornerRadius(6.0)
        ));
    });
}

fn spawn_units_panel(middle: &mut ChildSpawnerCommands) {
    middle
        .spawn((
            UnitsPanel,
            panel(UiStyle {
                width: Val::Px(240.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            })
        ))
        .with_children(|list| {
            list.spawn(rect(120.0, 8.0, PLACEHOLDER, 4.0));
            list.spawn(UiNode(UiStyle {
                height: Val::Px(6.0),
                ..default()
            }));
            for i in 0..8 {
                list.spawn((
                    UiNode(UiStyle {
                        height: Val::Px(40.0),
                        align_items: AlignItems::CENTER,
                        column_gap: Val::Px(10.0),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                        ..default()
                    }),
                    UiCornerRadius(6.0),
                    ButtonColors::ROW.bundle()
                ))
                .with_children(|row| {
                    row.spawn(rect(26.0, 26.0, RESOURCES[i % 4], 13.0));
                    row.spawn(UiNode(UiStyle {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        ..default()
                    }))
                    .with_children(|lines| {
                        lines.spawn(rect(60.0 + (i * 23 % 70) as f32, 7.0, PLACEHOLDER, 3.5));
                        lines.spawn(rect(40.0 + (i * 17 % 50) as f32, 5.0, OUTLINE, 2.5));
                    });
                });
            }
        });
}

fn spawn_minimap(middle: &mut ChildSpawnerCommands) {
    middle
        .spawn(panel(UiStyle {
            width: Val::Px(200.0),
            height: Val::Px(200.0),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        }))
        .with_children(|map| {
            let markers = [(20.0, 30.0), (55.0, 45.0), (70.0, 20.0), (35.0, 75.0)];
            for (i, (x, y)) in markers.into_iter().enumerate() {
                map.spawn((
                    UiNode(UiStyle {
                        position: UiPosition::Absolute,
                        inset: UiRect {
                            left: Val::Percent(x),
                            top: Val::Percent(y),
                            ..UiRect::all(Val::Auto)
                        },
                        width: Val::Px(12.0),
                        height: Val::Px(12.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    }),
                    UiBackground(RESOURCES[i % 4]),
                    UiBorderColor(WdeColor::WHITE),
                    UiCornerRadius(6.0)
                ));
            }
        });
}

fn spawn_toolbar(hud: &mut ChildSpawnerCommands) {
    hud.spawn(panel(UiStyle {
        align_self: Some(AlignSelf::CENTER),
        column_gap: Val::Px(8.0),
        padding: UiRect::all(Val::Px(8.0)),
        border: UiRect::all(Val::Px(1.0)),
        ..default()
    }))
    .with_children(|toolbar| {
        for i in 0..6 {
            toolbar
                .spawn((
                    ToolbarButton(i),
                    UiNode(UiStyle {
                        width: Val::Px(56.0),
                        height: Val::Px(56.0),
                        align_items: AlignItems::CENTER,
                        justify_content: JustifyContent::CENTER,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    }),
                    UiBorderColor(OUTLINE),
                    UiCornerRadius(10.0),
                    ButtonColors::TOOLBAR.bundle()
                ))
                // The icon has no `UiInteraction`: it forwards the pointer to the button
                .with_child(rect(24.0, 24.0, RESOURCES[i % 4], 4.0 + i as f32 * 2.0));
        }
    });
}

fn update_button_colors(
    mut buttons: Query<(&UiInteraction, &ButtonColors, &mut UiBackground), Changed<UiInteraction>>
) {
    for (interaction, colors, mut background) in &mut buttons {
        background.0 = match interaction {
            UiInteraction::None => colors.idle,
            UiInteraction::Hovered => colors.hovered,
            UiInteraction::Pressed => colors.pressed
        };
    }
}

fn handle_toolbar_clicks(
    mut clicks: MessageReader<UiClick>,
    buttons: Query<&ToolbarButton>,
    mut panel: Single<&mut UiNode, With<UnitsPanel>>
) {
    for click in clicks.read() {
        let Ok(ToolbarButton(index)) = buttons.get(click.entity) else {
            continue;
        };
        if *index == 0 {
            let style = &mut panel.0;
            style.display = match style.display {
                UiDisplay::None => UiDisplay::Flex,
                _ => UiDisplay::None
            };
            info!("Toggled the units panel.");
        } else {
            info!("Clicked toolbar button {}.", index);
        }
    }
}

fn animate_energy(time: Res<Time>, mut gauge: Single<&mut UiNode, With<EnergyLevel>>) {
    gauge.0.width = Val::Percent(60.0 + 35.0 * time.elapsed_secs().sin());
}
