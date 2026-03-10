use wde_camera::camera::CameraView;
use wde_logger::prelude::*;
use bevy::{
    input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    prelude::*,
};
use std::f32::consts::*;

pub(crate) struct ThirdPersonControllerPlugin;
impl Plugin for ThirdPersonControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update);
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct ThirdPersonController {
    pub enabled: bool,
    pub initialized: bool,
    pub sensitivity: f32,
    pub key_forward: KeyCode,
    pub key_back: KeyCode,
    pub key_left: KeyCode,
    pub key_right: KeyCode,
    pub key_up: KeyCode,
    pub key_down: KeyCode,
    pub key_run: KeyCode,
    pub mouse_key_rotate: MouseButton,
    pub min_move_speed: f32,
    pub max_move_speed: f32,
    pub run_speed_multiplier: f32,
    pub zoom_speed: f32,
    pub friction: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub distance: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub min_pitch: f32,
    pub max_pitch: f32,
    pub target: Vec3,
    pub velocity: Vec3,
    pub edge_scroll_enabled: bool,
    pub edge_scroll_distance: f32,
    pub edge_scroll_speed: f32,
}

impl Default for ThirdPersonController {
    fn default() -> Self {
        Self {
            enabled: true,
            initialized: false,
            sensitivity: 0.005,
            key_forward: KeyCode::KeyW,
            key_back: KeyCode::KeyS,
            key_left: KeyCode::KeyD,
            key_right: KeyCode::KeyA,
            key_up: KeyCode::KeyE,
            key_down: KeyCode::KeyQ,
            key_run: KeyCode::ShiftLeft,
            mouse_key_rotate: MouseButton::Middle,
            min_move_speed: 10.0,
            max_move_speed: 60.0,
            run_speed_multiplier: 2.5,
            zoom_speed: 2.5,
            friction: 0.5,
            pitch: 0.9,
            yaw: 0.0,
            distance: 30.0,
            min_distance: 10.0,
            max_distance: 60.0,
            min_pitch: 0.1,            // Nearly horizontal
            max_pitch: PI / 2.0 - 0.1, // Nearly vertical
            target: Vec3::ZERO,
            velocity: Vec3::ZERO,
            edge_scroll_enabled: !cfg!(debug_assertions),
            edge_scroll_distance: 5.0,
            edge_scroll_speed: 20.0,
        }
    }
}

// Update the camera controller
fn update(
    mut camera_query: Query<(&mut Transform, &mut ThirdPersonController), With<CameraView>>,
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut mouse_events: MessageReader<MouseMotion>,
    mut mouse_scroll_events: MessageReader<MouseWheel>,
    windows: Query<&Window>,
) {
    let dt = time.delta_secs();

    if let Ok((mut transform, mut controller)) = camera_query.single_mut() {
        if !controller.initialized {
            // Initialize target from current camera position projected onto ground
            controller.target = Vec3::new(transform.translation.x, 0.0, transform.translation.z);
            controller.initialized = true;
            info!("Third-person camera controller initialized.");
        }
        if !controller.enabled {
            mouse_events.clear();
            mouse_scroll_events.clear();
            return;
        }

        // Handle mouse scroll for zoom
        let mut scroll = 0.0;
        for scroll_event in mouse_scroll_events.read() {
            let amount = match scroll_event.unit {
                MouseScrollUnit::Line => scroll_event.y,
                MouseScrollUnit::Pixel => scroll_event.y / 16.0,
            };
            scroll += amount;
        }
        controller.distance -= scroll * controller.zoom_speed;
        controller.distance = controller
            .distance
            .clamp(controller.min_distance, controller.max_distance);

        // Handle middle mouse button for rotation
        let is_rotating = mouse_button_input.pressed(controller.mouse_key_rotate);
        let mut mouse_delta = Vec2::ZERO;
        if is_rotating {
            for mouse_event in mouse_events.read() {
                mouse_delta += mouse_event.delta;
            }
        } else {
            mouse_events.clear();
        }

        if mouse_delta != Vec2::ZERO {
            // Rotate around target
            controller.yaw -= mouse_delta.x * controller.sensitivity;
            controller.pitch += mouse_delta.y * controller.sensitivity;
            controller.pitch = controller
                .pitch
                .clamp(controller.min_pitch, controller.max_pitch);
        }

        // Handle keyboard input for moving the target
        let mut axis_input = Vec3::ZERO;
        if keyboard_input.pressed(controller.key_forward) {
            axis_input.z += 1.0;
        }
        if keyboard_input.pressed(controller.key_back) {
            axis_input.z -= 1.0;
        }
        if keyboard_input.pressed(controller.key_right) {
            axis_input.x += 1.0;
        }
        if keyboard_input.pressed(controller.key_left) {
            axis_input.x -= 1.0;
        }
        if keyboard_input.pressed(controller.key_up) {
            axis_input.y += 1.0;
        }
        if keyboard_input.pressed(controller.key_down) {
            axis_input.y -= 1.0;
        }

        // Handle edge-scrolling with mouse cursor
        if controller.edge_scroll_enabled
            && let Ok(window) = windows.single()
            && let Some(cursor_pos) = window.cursor_position()
        {
            let width = window.width();
            let height = window.height();
            let threshold = controller.edge_scroll_distance;

            // Check horizontal edges (left/right)
            if cursor_pos.x < threshold {
                let proximity = 1.0 - (cursor_pos.x / threshold);
                axis_input.x += proximity;
            } else if cursor_pos.x > width - threshold {
                let proximity = 1.0 - ((width - cursor_pos.x) / threshold);
                axis_input.x -= proximity;
            }

            // Check vertical edges (top/bottom)
            if cursor_pos.y < threshold {
                let proximity = 1.0 - (cursor_pos.y / threshold);
                axis_input.z += proximity;
            } else if cursor_pos.y > height - threshold {
                let proximity = 1.0 - ((height - cursor_pos.y) / threshold);
                axis_input.z -= proximity;
            }
        }

        // Apply movement to target
        if axis_input != Vec3::ZERO {
            // Calculate speed based on zoom distance (closer = slower, farther = faster)
            let distance_ratio = (controller.distance - controller.min_distance)
                / (controller.max_distance - controller.min_distance);
            let base_move_speed = controller.min_move_speed
                + distance_ratio * (controller.max_move_speed - controller.min_move_speed);
            let base_run_speed = base_move_speed * controller.run_speed_multiplier;

            let max_speed = if keyboard_input.pressed(controller.key_run) {
                base_run_speed
            } else {
                base_move_speed
            };
            controller.velocity = axis_input.normalize() * max_speed;
        } else {
            let friction = controller.friction.clamp(0.0, 1.0);
            controller.velocity *= 1.0 - friction;
            if controller.velocity.length_squared() < 1e-6 {
                controller.velocity = Vec3::ZERO;
            }
        }

        // Calculate movement direction based on camera yaw (horizontal rotation only)
        let yaw_rotation = Quat::from_rotation_y(controller.yaw);
        let forward = yaw_rotation * Vec3::Z;
        let right = yaw_rotation * Vec3::X;
        let velocity = controller.velocity;

        controller.target +=
            velocity.x * dt * right + velocity.y * dt * Vec3::Y + velocity.z * dt * forward;

        // Calculate camera position based on target, distance, and rotation
        let offset = Quat::from_euler(EulerRot::YXZ, controller.yaw, controller.pitch, 0.0)
            * Vec3::new(0.0, 0.0, -controller.distance);

        transform.translation = controller.target + offset;

        // Make camera look at target
        transform.look_at(controller.target, Vec3::Y);
    }
}
