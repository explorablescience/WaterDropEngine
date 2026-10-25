//! Pointer hit-testing against the laid out UI nodes.
use bevy::{prelude::*, window::PrimaryWindow};

use crate::node::{ComputedUiNode, UiBackground, UiImage};

/// Makes a [`UiNode`](crate::node::UiNode) react to the pointer. Descendants without their own
/// [`UiInteraction`] forward the pointer to it, so a button can hold decorations.
#[derive(Component, Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum UiInteraction {
    #[default]
    None,
    Hovered,
    /// Pressed on this node, until the button is released (even outside of it).
    Pressed
}

/// Sent when the left button is pressed and released on the same [`UiInteraction`] node.
#[derive(Message, Clone, Copy, Debug)]
pub struct UiClick {
    pub entity: Entity
}

/// Pointer state over the UI, for game systems to ignore input meant for it. Mouse presses over
/// the UI are also consumed: game systems don't see them as just pressed.
#[derive(Resource, Default, Debug)]
pub struct UiPointer {
    /// Whether the pointer is over a visible UI node.
    pub over_ui: bool,
    /// The [`UiInteraction`] node under the pointer.
    pub hovered: Option<Entity>,
    /// The [`UiInteraction`] node the left button was pressed on.
    pub pressed: Option<Entity>
}

/// Nodes that are drawn, so block the pointer.
type Visible = Or<(With<UiBackground>, With<UiImage>)>;

pub(crate) fn update_interactions(
    window: Single<&Window, With<PrimaryWindow>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    nodes: Query<(Entity, &ComputedUiNode), Visible>,
    parents: Query<&ChildOf>,
    mut interactions: Query<(Entity, &mut UiInteraction)>,
    mut pointer: ResMut<UiPointer>,
    mut clicks: MessageWriter<UiClick>
) {
    let topmost = window.cursor_position().and_then(|cursor| {
        nodes
            .iter()
            .filter(|(_, node)| node.rect().contains(cursor))
            .max_by_key(|(_, node)| node.order)
            .map(|(entity, _)| entity)
    });
    let target = topmost.and_then(|entity| {
        std::iter::once(entity)
            .chain(parents.iter_ancestors(entity))
            .find(|e| interactions.contains(*e))
    });
    pointer.over_ui = topmost.is_some();
    pointer.hovered = target;

    if mouse.just_pressed(MouseButton::Left) {
        pointer.pressed = target;
    }
    if mouse.just_released(MouseButton::Left)
        && let Some(pressed) = pointer.pressed.take()
        && target == Some(pressed)
    {
        clicks.write(UiClick { entity: pressed });
    }

    for (entity, mut interaction) in &mut interactions {
        let state = if pointer.pressed == Some(entity) {
            UiInteraction::Pressed
        } else if target == Some(entity) && pointer.pressed.is_none() {
            UiInteraction::Hovered
        } else {
            UiInteraction::None
        };
        interaction.set_if_neq(state);
    }

    if pointer.over_ui {
        for button in [MouseButton::Left, MouseButton::Right, MouseButton::Middle] {
            mouse.clear_just_pressed(button);
        }
    }
}
