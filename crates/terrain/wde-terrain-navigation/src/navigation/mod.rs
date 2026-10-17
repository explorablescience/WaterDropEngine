use std::collections::{HashMap, VecDeque};
use wde_logger::prelude::*;

use bevy::prelude::*;
use wde_terrain_grid::prelude::*;

use crate::{TerrainNavigator, navgrid::NavGrid};

mod pathfinding;

pub use pathfinding::AIRLOCK_ROLE;
use pathfinding::{NavWorld, Waypoint, find_path};

/// Turning speed, in radians per second.
const TURN_SPEED: f32 = 10.0;
/// Distance at which a waypoint counts as reached.
const WAYPOINT_REACHED: f32 = 0.05;

/// Walking speeds: a base speed scaled by a factor depending on the terrain walked on. The path
/// search minimizes the walking time, so colonists prefer paths and tunnels over bare ground.
#[derive(Resource, Clone, Debug)]
pub struct WalkSpeeds {
    /// Speed for a factor of 1, in world units per second (about two colonist heights).
    pub base: f32,
    /// Factor inside pressurized buildings and tunnels.
    pub indoor: f32,
    /// Factor on outdoor paths and paved accesses.
    pub path: f32,
    /// Factor on bare outdoor ground.
    pub ground: f32,
    /// Time spent cycling an airlock when going out or in, in seconds.
    pub airlock_cycle: f32
}
impl Default for WalkSpeeds {
    fn default() -> Self {
        Self {
            base: 1.5,
            indoor: 1.2,
            path: 1.0,
            ground: 0.7,
            airlock_cycle: 2.5
        }
    }
}

/// Whether a walking entity is in a pressurized place. It flips once an airlock cycle is over,
/// e.g. to swap a colonist between its suit and indoor models.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Indoors(pub bool);

pub struct NavigationPlugin;
impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WalkSpeeds>().add_systems(
            Update,
            (request_paths, replan_on_network_change, follow_paths).chain()
        );
    }
}

/// Path followed by a moving entity.
#[derive(Component)]
pub(crate) struct NavPath {
    pub waypoints: VecDeque<Waypoint>,
    pub destination: Vec3,
    /// Remaining stop time at the waypoint just reached (airlock cycle).
    pub waiting: f32
}

fn nav_world<'a>(
    grid: &'a Grid,
    placed: &'a Query<(Entity, &GridEntity), With<BuildingConnections>>
) -> NavWorld<'a> {
    NavWorld {
        grid,
        buildings: placed.iter().collect::<HashMap<_, _>>()
    }
}

/// Computes the paths requested through the [`TerrainNavigator`].
fn request_paths(
    mut commands: Commands,
    mut navigator: ResMut<TerrainNavigator>,
    transforms: Query<&Transform>,
    grid: Res<Grid>,
    placed: Query<(Entity, &GridEntity), With<BuildingConnections>>,
    speeds: Res<WalkSpeeds>
) {
    if navigator.new_entities.is_empty() && navigator.removed_entities.is_empty() {
        return;
    }
    let world = nav_world(&grid, &placed);
    for (entity, destination) in navigator.new_entities.drain(..) {
        let Ok(transform) = transforms.get(entity) else {
            continue;
        };
        match find_path(&world, &speeds, transform.translation, destination) {
            Some(waypoints) => {
                commands.entity(entity).insert(NavPath {
                    waypoints: waypoints.into(),
                    destination,
                    waiting: 0.0
                });
            }
            None => {
                warn!("No path from {} to {}.", transform.translation, destination);
                commands.entity(entity).remove::<NavPath>();
            }
        }
    }
    for entity in navigator.removed_entities.drain(..) {
        commands.entity(entity).remove::<NavPath>();
    }
}

/// Replans the moving entities when buildings are placed or removed.
fn replan_on_network_change(
    nav_grid: Res<NavGrid>,
    mut moving: Query<(&Transform, &mut NavPath)>,
    grid: Res<Grid>,
    placed: Query<(Entity, &GridEntity), With<BuildingConnections>>,
    speeds: Res<WalkSpeeds>
) {
    if !nav_grid.is_changed() || moving.is_empty() {
        return;
    }
    let world = nav_world(&grid, &placed);
    for (transform, mut path) in &mut moving {
        if path.waiting > 0.0 {
            continue; // Let the airlock cycle finish
        }
        // Keep the current path when the destination became unreachable
        if let Some(waypoints) = find_path(&world, &speeds, transform.translation, path.destination)
        {
            path.waypoints = waypoints.into();
        }
    }
}

/// Moves entities along their path at the speed of the terrain, stopping at airlocks, and turning
/// smoothly toward their heading.
fn follow_paths(
    mut commands: Commands,
    mut moving: Query<(Entity, &mut Transform, &mut NavPath, Option<&Indoors>)>,
    speeds: Res<WalkSpeeds>,
    time: Res<Time>
) {
    let dt = time.delta_secs();
    for (entity, mut transform, mut path, indoors) in &mut moving {
        let mut time_left = dt;
        let mut heading = None;
        let mut environment = indoors.map(|i| i.0);
        while time_left > 0.0 {
            // Airlock cycle: stand still
            if path.waiting > 0.0 {
                let waited = path.waiting.min(time_left);
                path.waiting -= waited;
                time_left -= waited;
                continue;
            }
            let Some(&waypoint) = path.waypoints.front() else {
                break;
            };
            // Helmet on or off for the segment ahead (after any airlock cycle)
            environment = Some(waypoint.indoor);

            let speed = speeds.base * waypoint.speed;
            let to_target = waypoint.position - transform.translation;
            let distance = to_target.length();
            let reach = speed * time_left;
            if distance <= WAYPOINT_REACHED.max(reach) {
                transform.translation = waypoint.position;
                time_left -= distance / speed.max(1e-3);
                path.waypoints.pop_front();
                path.waiting = waypoint.wait;
                continue;
            }
            transform.translation += to_target / distance * reach;
            heading = Some(to_target);
            time_left = 0.0;
        }

        if let Some(indoor) = environment
            && indoors.is_none_or(|i| i.0 != indoor)
        {
            commands.entity(entity).insert(Indoors(indoor));
        }

        // Models face +X: yaw so that +X follows the horizontal heading
        if let Some(heading) = heading
            && heading.xz().length_squared() > 1e-6
        {
            let target = Quat::from_rotation_y((-heading.z).atan2(heading.x));
            let rotation = transform.rotation;
            transform.rotation = rotation.slerp(target, (TURN_SPEED * dt).min(1.0));
        }

        if path.waypoints.is_empty() && path.waiting <= 0.0 {
            commands.entity(entity).remove::<NavPath>();
        }
    }
}
