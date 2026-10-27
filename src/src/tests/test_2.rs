//! Colonist info panel: shows the selected colonist's state, built with `wde_ui`.
use bevy::prelude::*;
use wde::prelude::{Color as WdeColor, *};

use crate::test::{EntitySelectedMarker, WALK_MIN_SPEED, WalkTracker};

pub struct ColonistInfoPlugin;
impl Plugin for ColonistInfoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_info_panel)
            .add_systems(Update, update_info_panel);
    }
}

const PANEL: WdeColor = WdeColor::Srgba(0.07, 0.08, 0.1, 0.9);
const INSET: WdeColor = WdeColor::Srgba(0.15, 0.17, 0.2, 1.0);
const OUTLINE: WdeColor = WdeColor::Srgba(0.25, 0.28, 0.33, 1.0);
const MUTED: WdeColor = WdeColor::Srgba(0.6, 0.64, 0.7, 1.0);
const INDOORS: WdeColor = WdeColor::Srgba(0.35, 0.8, 1.0, 1.0);
const OUTDOORS: WdeColor = WdeColor::Srgba(1.0, 0.6, 0.25, 1.0);
/// Speed of a full speed bar, in m/s.
const SPEED_BAR_MAX: f32 = 2.0;

#[derive(Component)]
struct InfoPanel;

#[derive(Component)]
struct Avatar;

#[derive(Component)]
struct SpeedFill;

/// A text of the panel, filled from the selected colonist.
#[derive(Component, Clone, Copy)]
enum InfoField {
    Name,
    Subtitle,
    Status,
    Location,
    Position,
    Speed
}

fn spawn_info_panel(mut commands: Commands) {
    commands
        .spawn((
            Name::new("Colonist Info"),
            InfoPanel,
            UiNode(UiStyle {
                display: UiDisplay::None,
                position: UiPosition::Absolute,
                // Below the editor menu bar
                inset: UiRect {
                    right: Val::Px(12.0),
                    top: Val::Px(60.0),
                    ..UiRect::all(Val::Auto)
                },
                width: Val::Px(280.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                padding: UiRect::all(Val::Px(14.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            }),
            UiBackground(PANEL),
            UiBorderColor(OUTLINE),
            UiCornerRadius(12.0)
        ))
        .with_children(|panel| {
            // Header: avatar, name and subtitle
            panel
                .spawn(UiNode(UiStyle {
                    align_items: AlignItems::CENTER,
                    column_gap: Val::Px(12.0),
                    ..default()
                }))
                .with_children(|header| {
                    header.spawn((
                        Avatar,
                        UiNode(UiStyle {
                            width: Val::Px(36.0),
                            height: Val::Px(36.0),
                            flex_shrink: 0.0,
                            ..default()
                        }),
                        UiBackground(INDOORS),
                        UiCornerRadius(18.0)
                    ));
                    header
                        .spawn(UiNode(UiStyle {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(2.0),
                            ..default()
                        }))
                        .with_children(|titles| {
                            titles.spawn((InfoField::Name, UiText::new("").with_size(18.0).bold()));
                            titles.spawn((
                                InfoField::Subtitle,
                                UiText::new("").with_size(13.0).with_color(MUTED)
                            ));
                        });
                });

            panel.spawn((
                UiNode(UiStyle {
                    height: Val::Px(1.0),
                    ..default()
                }),
                UiBackground(OUTLINE)
            ));

            for (label, field) in [
                ("Status", InfoField::Status),
                ("Location", InfoField::Location),
                ("Position", InfoField::Position),
                ("Speed", InfoField::Speed)
            ] {
                panel
                    .spawn(UiNode(UiStyle {
                        justify_content: JustifyContent::SPACE_BETWEEN,
                        ..default()
                    }))
                    .with_children(|row| {
                        row.spawn(UiText::new(label).with_size(14.0).with_color(MUTED));
                        row.spawn((field, UiText::new("").with_size(14.0)));
                    });
            }

            panel
                .spawn((
                    UiNode(UiStyle {
                        height: Val::Px(6.0),
                        ..default()
                    }),
                    UiBackground(INSET),
                    UiCornerRadius(3.0)
                ))
                .with_child((
                    SpeedFill,
                    UiNode(UiStyle {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    }),
                    UiBackground(INDOORS),
                    UiCornerRadius(3.0)
                ));

            panel.spawn(
                UiText::new("Right-click the ground to send them there.")
                    .with_size(12.0)
                    .with_color(MUTED)
            );
        });
}

fn update_info_panel(
    selected: Query<(&Name, &Transform, &Indoors, &WalkTracker), With<EntitySelectedMarker>>,
    mut panel: Single<&mut UiNode, (With<InfoPanel>, Without<SpeedFill>)>,
    mut speed_fill: Single<(&mut UiNode, &mut UiBackground), (With<SpeedFill>, Without<InfoPanel>)>,
    mut avatar: Single<&mut UiBackground, (With<Avatar>, Without<SpeedFill>)>,
    mut fields: Query<(&InfoField, &mut UiText)>
) {
    // Show the first selected colonist by name, so the panel doesn't jump between them
    let Some((name, transform, indoors, walk)) = selected
        .iter()
        .min_by(|a, b| a.0.as_str().cmp(b.0.as_str()))
    else {
        if panel.0.display != UiDisplay::None {
            panel.0.display = UiDisplay::None;
        }
        return;
    };
    if panel.0.display == UiDisplay::None {
        panel.0.display = UiDisplay::Flex;
    }

    let count = selected.iter().count();
    let walking = walk.speed > WALK_MIN_SPEED;
    let accent = if indoors.0 { INDOORS } else { OUTDOORS };
    for (field, mut text) in &mut fields {
        let value = match field {
            InfoField::Name => name.to_string(),
            InfoField::Subtitle if count > 1 => format!("Colonist · {} selected", count),
            InfoField::Subtitle => "Colonist".to_string(),
            InfoField::Status if walking => "Walking".to_string(),
            InfoField::Status => "Idle".to_string(),
            InfoField::Location if indoors.0 => "Indoors".to_string(),
            InfoField::Location => "Outdoors (suit)".to_string(),
            InfoField::Position => format!(
                "{:.1}, {:.1}",
                transform.translation.x, transform.translation.z
            ),
            InfoField::Speed => format!("{:.2} m/s", if walking { walk.speed } else { 0.0 })
        };
        if text.text != value {
            text.text = value;
        }
    }

    avatar.0 = accent;
    let (fill_node, fill_color) = &mut *speed_fill;
    fill_color.0 = accent;
    let speed = if walking { walk.speed } else { 0.0 };
    let width = Val::Percent((speed / SPEED_BAR_MAX).clamp(0.0, 1.0) * 100.0);
    if fill_node.0.width != width {
        fill_node.0.width = width;
    }
}
