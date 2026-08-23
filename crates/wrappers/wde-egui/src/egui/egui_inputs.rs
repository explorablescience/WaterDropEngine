use super::egui_context::EguiContext;
use bevy::{
    input::{
        ButtonState,
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
        prelude::*
    },
    prelude::*
};

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
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_input(
    windows: Query<&Window>,
    mut egui_inputs: ResMut<EguiInputs>,
    mut keyboard_input_messages: MessageReader<KeyboardInput>,
    mut mouse_button_input_messages: MessageReader<MouseButtonInput>,
    mut mouse_motion_messages: MessageReader<MouseMotion>,
    mut mouse_wheel_messages: MessageReader<MouseWheel>,
    keyboard_input: Res<ButtonInput<KeyCode>>
) {
    let mut raw_input = egui::RawInput::default();

    // Get mouse position
    let window = windows.iter().next().unwrap();

    // Report the real window size and scale factor, so egui can lay out full-screen panels
    // (e.g. `CentralPanel`) correctly. Without this, `screen_rect` stays `None` and egui falls
    // back to an arbitrary large default screen size, unrelated to the actual window.
    raw_input.screen_rect = Some(egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::vec2(window.width(), window.height())
    ));
    raw_input.viewports.insert(
        raw_input.viewport_id,
        egui::ViewportInfo {
            native_pixels_per_point: Some(window.scale_factor()),
            ..Default::default()
        }
    );

    let mouse_position = window
        .cursor_position()
        .map(|pos| egui::Pos2 { x: pos.x, y: pos.y });

    // Add pointer position event if we have cursor position
    if let Some(pos) = mouse_position {
        raw_input.events.push(egui::Event::PointerMoved(pos));
    }

    // Handle mouse motion for continuous tracking
    for _motion in mouse_motion_messages.read() {
        // Motion events are handled by the PointerMoved event above
        // We just need to consume the messages here
    }

    // Track modifiers. Must happen before forwarding wheel/button/key events below, since those
    // read `raw_input.modifiers` (e.g. Ctrl+scroll-to-zoom depends on it being set correctly).
    let modifiers = egui::Modifiers {
        ctrl: keyboard_input.pressed(KeyCode::ControlLeft)
            || keyboard_input.pressed(KeyCode::ControlRight),
        shift: keyboard_input.pressed(KeyCode::ShiftLeft)
            || keyboard_input.pressed(KeyCode::ShiftRight),
        alt: keyboard_input.pressed(KeyCode::AltLeft) || keyboard_input.pressed(KeyCode::AltRight),
        mac_cmd: false, // Can add Mac support if needed
        command: keyboard_input.pressed(KeyCode::ControlLeft)
            || keyboard_input.pressed(KeyCode::ControlRight)
    };
    raw_input.modifiers = modifiers;

    // Forward wheel scrolling to egui.
    for event in mouse_wheel_messages.read() {
        let unit = match event.unit {
            bevy::input::mouse::MouseScrollUnit::Line => egui::MouseWheelUnit::Line,
            bevy::input::mouse::MouseScrollUnit::Pixel => egui::MouseWheelUnit::Point
        };

        raw_input.events.push(egui::Event::MouseWheel {
            unit,
            delta: egui::Vec2::new(event.x, event.y),
            modifiers: raw_input.modifiers
        });
    }

    // Handle mouse buttons
    for event in mouse_button_input_messages.read() {
        let pointer_button = match event.button {
            MouseButton::Left => Some(egui::PointerButton::Primary),
            MouseButton::Right => Some(egui::PointerButton::Secondary),
            MouseButton::Middle => Some(egui::PointerButton::Middle),
            _ => None
        };

        if let Some(button) = pointer_button {
            match event.state {
                ButtonState::Pressed => {
                    raw_input.events.push(egui::Event::PointerButton {
                        pos: mouse_position.unwrap_or_default(),
                        button,
                        pressed: true,
                        modifiers
                    });
                }
                ButtonState::Released => {
                    raw_input.events.push(egui::Event::PointerButton {
                        pos: mouse_position.unwrap_or_default(),
                        button,
                        pressed: false,
                        modifiers
                    });
                }
            }
        }
    }

    // Handle keyboard input
    for event in keyboard_input_messages.read() {
        // Handle text input from logical_key
        if event.state == ButtonState::Pressed {
            if let bevy::input::keyboard::Key::Character(ref s) = event.logical_key {
                // Send text to egui for typing in text fields
                raw_input.events.push(egui::Event::Text(s.to_string()));
            }

            // Also send key events for special keys
            if let Some(key) = bevy_to_egui_key(event.key_code) {
                raw_input.events.push(egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    physical_key: None
                });
            }
        } else if event.state == ButtonState::Released
            && let Some(key) = bevy_to_egui_key(event.key_code)
        {
            raw_input.events.push(egui::Event::Key {
                key,
                pressed: false,
                repeat: false,
                modifiers,
                physical_key: None
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
    mut mouse_motion_messages: ResMut<Messages<MouseMotion>>
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
        KeyCode::ShiftRight
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
        KeyCode::Insert => Some(egui::Key::Insert),

        // Letters for shortcuts (Ctrl+A, Ctrl+C, etc.)
        KeyCode::KeyA => Some(egui::Key::A),
        KeyCode::KeyB => Some(egui::Key::B),
        KeyCode::KeyC => Some(egui::Key::C),
        KeyCode::KeyD => Some(egui::Key::D),
        KeyCode::KeyE => Some(egui::Key::E),
        KeyCode::KeyF => Some(egui::Key::F),
        KeyCode::KeyG => Some(egui::Key::G),
        KeyCode::KeyH => Some(egui::Key::H),
        KeyCode::KeyI => Some(egui::Key::I),
        KeyCode::KeyJ => Some(egui::Key::J),
        KeyCode::KeyK => Some(egui::Key::K),
        KeyCode::KeyL => Some(egui::Key::L),
        KeyCode::KeyM => Some(egui::Key::M),
        KeyCode::KeyN => Some(egui::Key::N),
        KeyCode::KeyO => Some(egui::Key::O),
        KeyCode::KeyP => Some(egui::Key::P),
        KeyCode::KeyQ => Some(egui::Key::Q),
        KeyCode::KeyR => Some(egui::Key::R),
        KeyCode::KeyS => Some(egui::Key::S),
        KeyCode::KeyT => Some(egui::Key::T),
        KeyCode::KeyU => Some(egui::Key::U),
        KeyCode::KeyV => Some(egui::Key::V),
        KeyCode::KeyW => Some(egui::Key::W),
        KeyCode::KeyX => Some(egui::Key::X),
        KeyCode::KeyY => Some(egui::Key::Y),
        KeyCode::KeyZ => Some(egui::Key::Z),

        _ => None
    }
}
