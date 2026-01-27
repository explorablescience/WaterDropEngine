use bevy::{input::{ButtonState, keyboard::KeyboardInput, mouse::MouseButtonInput, prelude::*}, prelude::*};

/// Resource to store egui inputs
#[derive(Resource, Default)]
pub(crate) struct EguiInputs(pub egui::RawInput);

/// Plugin to handle egui inputs
pub(crate) struct EguiInputsPlugin;
impl Plugin for EguiInputsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EguiInputs>();
    }
}

/// System to handle Bevy input events and convert them to egui inputs
pub(crate) fn handle_input(
    mut mouse_button_events: MessageReader<MouseButtonInput>,
    mut keyboard_events: MessageReader<KeyboardInput>,
    windows: Query<&Window>,
    mut egui_inputs: ResMut<EguiInputs>,
) {
    let mut raw_input = egui::RawInput::default();

    // Get mouse position
    let window = windows.iter().next().unwrap();
    let mouse_position = window.cursor_position().map(|pos| egui::Pos2 { x: pos.x, y: pos.y });

    // Handle mouse buttons
    for event in mouse_button_events.read() {
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
    for event in keyboard_events.read() {
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
