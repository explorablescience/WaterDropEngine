use bevy::{input::{ButtonState, keyboard::KeyboardInput, mouse::{MouseButtonInput, MouseMotion, MouseWheel}, prelude::*}, prelude::*};
use super::egui_context::EguiContext;

/// Resource to store egui inputs
#[derive(Resource, Default)]
pub(crate) struct EguiInputs(pub egui::RawInput);

/// Plugin to handle egui inputs
pub(crate) struct EguiInputsPlugin;
impl Plugin for EguiInputsPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<EguiInputs>();
    }
}

/// System to handle Bevy input events and convert them to egui inputs
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_input(
    windows: Query<&Window>,
    mut egui_inputs: ResMut<EguiInputs>,
    mut keyboard_input_messages: MessageReader<KeyboardInput>,
    mut mouse_button_input_messages: MessageReader<MouseButtonInput>,
    mut mouse_motion_messages: MessageReader<MouseMotion>,
) {
    let mut raw_input = egui::RawInput::default();

    // Get mouse position
    let window = windows.iter().next().unwrap();
    let mouse_position = window.cursor_position().map(|pos| egui::Pos2 { x: pos.x, y: pos.y });

    // Add pointer position event if we have cursor position
    if let Some(pos) = mouse_position {
        raw_input.events.push(egui::Event::PointerMoved(pos));
    }

    // Handle mouse motion for continuous tracking
    for _motion in mouse_motion_messages.read() {
        // Motion events are handled by the PointerMoved event above
        // We just need to consume the messages here
    }

    // Handle mouse buttons
    for event in mouse_button_input_messages.read() {
        let pointer_button = match event.button {
            MouseButton::Left => Some(egui::PointerButton::Primary),
            MouseButton::Right => Some(egui::PointerButton::Secondary),
            MouseButton::Middle => Some(egui::PointerButton::Middle),
            _ => None,
        };

        if let Some(button) = pointer_button {
            match event.state {
                ButtonState::Pressed => {
                    raw_input.events.push(egui::Event::PointerButton {
                        pos: mouse_position.unwrap_or_default(),
                        button,
                        pressed: true,
                        modifiers: egui::Modifiers::default(),
                    });
                }
                ButtonState::Released => {
                    raw_input.events.push(egui::Event::PointerButton {
                        pos: mouse_position.unwrap_or_default(),
                        button,
                        pressed: false,
                        modifiers: egui::Modifiers::default(),
                    });
                }
            }
        }
    }

    // Handle keyboard input
    for event in keyboard_input_messages.read() {
        if event.state == ButtonState::Pressed {
            if let Some(key) = bevy_to_egui_key(event.key_code) {
                raw_input.events.push(egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::default(),
                    physical_key: None,
                });
            }
        } else if event.state == ButtonState::Released
            && let Some(key) = bevy_to_egui_key(event.key_code) {
                raw_input.events.push(egui::Event::Key {
                    key,
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::default(),
                    physical_key: None,
                });
            }
    }
    egui_inputs.0 = raw_input;
}

pub(crate) fn clear_egui_inputs(
    egui_ctx: Res<EguiContext>,
    mut mouse_input: ResMut<ButtonInput<MouseButton>>,
    mut keyboard_input: ResMut<ButtonInput<KeyCode>>,
    mut keyboard_input_messages: ResMut<Messages<KeyboardInput>>,
    mut mouse_wheel_messages: ResMut<Messages<MouseWheel>>,
    mut mouse_button_input_messages: ResMut<Messages<MouseButtonInput>>,
    mut mouse_motion_messages: ResMut<Messages<MouseMotion>>,
) {
    // Check if egui wants keyboard or pointer input (now accurate since begin_pass was called)
    let egui_wants_keyboard = egui_ctx.0.wants_keyboard_input();
    let egui_wants_pointer = egui_ctx.0.wants_pointer_input() || egui_ctx.0.is_pointer_over_area();

    // Clear the input events after processing
    let modifiers = [
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::ShiftLeft,
        KeyCode::ShiftRight,
    ];
    let pressed = modifiers.map(|key| keyboard_input.pressed(key).then_some(key));
    if egui_wants_keyboard {
        keyboard_input.reset_all();
        keyboard_input_messages.clear();
    }
    if egui_wants_pointer {
        mouse_input.reset_all();
        mouse_wheel_messages.clear();
        mouse_button_input_messages.clear();
        mouse_motion_messages.clear();
    }
    for key in pressed.into_iter().flatten() {
        keyboard_input.press(key);
    }
}

/// Convert Bevy KeyCode to egui Key
/// Returns None if the key is not mapped
fn bevy_to_egui_key(key: KeyCode) -> Option<egui::Key> {
    match key {
        KeyCode::ArrowUp => Some(egui::Key::ArrowUp),
        KeyCode::ArrowDown => Some(egui::Key::ArrowDown),
        KeyCode::ArrowLeft => Some(egui::Key::ArrowLeft),
        KeyCode::ArrowRight => Some(egui::Key::ArrowRight),
        KeyCode::Escape => Some(egui::Key::Escape),
        KeyCode::Tab => Some(egui::Key::Tab),
        KeyCode::Backspace => Some(egui::Key::Backspace),
        KeyCode::Enter => Some(egui::Key::Enter),
        KeyCode::Space => Some(egui::Key::Space),
        KeyCode::Delete => Some(egui::Key::Delete),
        KeyCode::Home => Some(egui::Key::Home),
        KeyCode::End => Some(egui::Key::End),
        KeyCode::PageUp => Some(egui::Key::PageUp),
        KeyCode::PageDown => Some(egui::Key::PageDown),
        _ => None,
    }
}
