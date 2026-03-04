use std::collections::HashSet;

use wde_physics::prelude::*;
use wde_camera::prelude::*;
use wde_terrain::prelude::*;
use bevy::{input::{ButtonState, mouse::MouseButtonInput}, prelude::*, window::PrimaryWindow};

use crate::paint::brush::{PaintCommand, PaintBrush};

// Number of commands after which we automatically flush
const FLUSH_ON_N_COMMANDS: usize = 50;
// Maximum time (in seconds) before we automatically flush commands
const MAX_T_BEFORE_FLUSH: f32 = 0.05;
// Minimum time (in seconds) between two commands to avoid adding too many commands per second
const MIN_DT_BETWEEN_COMMANDS: f32 = 0.01;

pub struct PaintManagerPlugin;
impl Plugin for PaintManagerPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<PaintManager>()
            .add_systems(PreUpdate, flush_commands)
            .add_systems(Update, add_paint_command.chain());
    }
}

/// Resource that manages the painting commands and state of the painting process
/// It is used to store the current painting commands and the state of the painting process (active, painting, etc.)
/// When the commands are ready to be applied, they are moved to the to_extract field and the commands list is cleared
#[derive(Resource, Default)]
pub struct PaintManager {
    // True if active (allowing painting), false if not
    pub active: bool,

    // True if it is currently painting (mouse button held down), false if not
    pub painting: bool,
    // The last position where a paint command was added
    pub last_paint_position: Option<Vec3>,

    // True if should apply all painting commands on next iteration
    pub should_flush: bool,
    // List of painting commands
    pub commands: Option<Vec<PaintCommand>>,
    // List of chunks ids that should be updated on next iteration
    pub commands_chunks: Option<HashSet<IVec2>>,

    // List of commands and chunks that will be extracted on next extraction
    pub to_extract: Option<(Vec<PaintCommand>, HashSet<IVec2>)>
}
impl PaintManager {
    pub fn deactivate(&mut self) {
        self.painting = false;
        self.last_paint_position = None;
        self.should_flush = self.commands.is_some();
    }

    pub fn clear_commands(&mut self) {
        self.commands = None;
        self.commands_chunks = None;
        self.should_flush = false;
    }
}


/// Check for mouse input and add paint commands to the list
#[allow(clippy::too_many_arguments)]
fn add_paint_command(
    phworld: Res<PhysicsWorld>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Transform, &CameraView), With<Camera>>,
    mut mouse_input: MessageReader<MouseButtonInput>,
    mut paint_manager: ResMut<PaintManager>,
    brush_query: Query<&PaintBrush>,
    terrain: Query<&Terrain>,
    time: Res<Time>,
    mut last_update: Local<Option<f32>>,
    mut last_flush: Local<Option<f32>>,
) {
    // Check if active
    if !paint_manager.active {
        paint_manager.deactivate();
        return;
    }

    // Only add command on left mouse button press
    for event in mouse_input.read() {
        if event.button == MouseButton::Left && event.state == ButtonState::Pressed {
            paint_manager.painting = true;
        } else if event.button == MouseButton::Left && event.state == ButtonState::Released {
            paint_manager.painting = false;
        }
    }
    if !paint_manager.painting {
        paint_manager.deactivate();
        return;
    }

    // Check timer to avoid adding too many commands per second
    if last_update.is_some_and(|t| time.elapsed_secs() - t < MIN_DT_BETWEEN_COMMANDS) {
        return;
    }
    *last_update = Some(time.elapsed_secs());

    // Check if we have a brush and terrain
    let Ok(brush) = brush_query.single() else { return };
    let Ok(terrain) = terrain.single() else { return };

    // Get cursor position in NDC
    let Some(cursor_pos) = window.cursor_position() else { return };
    let cursor_ndc = cursor_pos / window.size();

    // Get camera data
    let Ok((camera_transform, camera_view)) = camera_query.single() else { return };
    let aspect_ratio = window.size().x / window.size().y;

    // Create ray from camera
    let ray = Ray::from_ndc(cursor_ndc, aspect_ratio, camera_transform, camera_view);

    // Cast the ray
    if let Some((_, toi)) = phworld.cast_ray(&ray, &RayCastConfig::default()) {
        // Get paint position
        let paint_pos = ray.point_at(toi);

        // Compute new position (interpolate)
        let neighbors = [
            IVec2::new(-1, -1), IVec2::new(0, -1), IVec2::new(1, -1),
            IVec2::new(-1, 0), IVec2::new(0, 0), IVec2::new(1, 0),
            IVec2::new(-1, 1), IVec2::new(0, 1), IVec2::new(1, 1),
        ];
        if let Some(last_pos) = paint_manager.last_paint_position {
            let distance = last_pos.distance(paint_pos);
            let spacing = brush.radius * 0.5; // Space commands by half the brush radius
            let mut steps = (distance / spacing).ceil() as u32;
            if steps > 20 {
                steps = 20; // Cap the number of interpolation steps to avoid too many commands
            }
            for i in 1..=steps {
                let t = i as f32 / steps as f32;
                let interp_pos = last_pos.lerp(paint_pos, t);
                match terrain.get_tile_idx_for_world_pos(interp_pos) {
                    Some(pos) => {
                        paint_manager.commands.get_or_insert_with(Vec::new).push(brush.paint(interp_pos));
                        for neighbor in neighbors {
                            paint_manager.commands_chunks.get_or_insert_with(HashSet::new).insert(pos + neighbor);
                        }
                    },
                    None => continue
                }
            }
        }

        // Add command
        let tile_pos = match terrain.get_tile_idx_for_world_pos(paint_pos) {
            Some(pos) => pos,
            None => return
        };
        paint_manager.commands.get_or_insert_with(Vec::new).push(brush.paint(paint_pos));
        for neighbor in neighbors {
            paint_manager.commands_chunks.get_or_insert_with(HashSet::new).insert(tile_pos + neighbor);
        }
        paint_manager.last_paint_position = Some(paint_pos);
    }

    // If more than x commands, flush to avoid memory issues
    if paint_manager.commands.as_ref().is_some_and(|c| c.len() > FLUSH_ON_N_COMMANDS)
        || (last_flush.is_none_or(|t| time.elapsed_secs() - t > MAX_T_BEFORE_FLUSH && paint_manager.commands.is_some())) {
        paint_manager.should_flush = true;
        *last_flush = Some(time.elapsed_secs());
    }
}

/// Flush commands to extract resource and clear the commands list
/// Each time it is called, the to_extract resource is cleared
/// If should_flush is true, the current commands are moved to to_extract and the commands list is cleared
fn flush_commands(mut paint_manager: ResMut<PaintManager>) {
    paint_manager.to_extract = None;
    if !paint_manager.should_flush {
        return;
    }

    // Add commands to extract list
    paint_manager.to_extract = Some((
        paint_manager.commands.take().unwrap_or_default(),
        paint_manager.commands_chunks.take().unwrap_or_default()
    ));

    // Clear commands
    paint_manager.clear_commands();
    paint_manager.should_flush = false;
}
