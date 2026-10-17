use std::collections::{HashMap, VecDeque};
use wde_logger::prelude::*;

use bevy::{ecs::system::SystemParam, prelude::*};
use wde_terrain_grid::prelude::*;

use crate::{TerrainNavigator, navgrid::NavGrid};

pub(crate) mod pathfinding;
pub(crate) mod walkmap;

pub use pathfinding::AIRLOCK_ROLE;
use pathfinding::{NavWorld, Waypoint, find_path, formation_spots};
use walkmap::WalkMap;

/// Turning speed, in radians per second.
const TURN_SPEED: f32 = 10.0;
/// Distance at which a waypoint counts as reached.
const WAYPOINT_REACHED: f32 = 0.05;
/// Distance to the center of an airlock within which its cycle starts: several colonists cycle
/// together rather than queueing for the exact center.
const AIRLOCK_REACHED: f32 = 0.3;
/// Time without moving after which a blocked walker looks for another path, in seconds.
const STUCK_REPLAN: f32 = 1.0;
/// Radius of entities without [`NavAgent`] when spreading a group.
const DEFAULT_AGENT_RADIUS: f32 = 0.14;
/// Spacing of group standing spots, in agent diameters.
const FORMATION_SPACING: f32 = 1.2;
/// How fast overlapping agents are pushed apart (fraction of the overlap per second).
const SEPARATION_RATE: f32 = 8.0;
/// Share of the push taken by a walking agent against a standing one: standing agents make way.
const WALKER_YIELD: f32 = 0.25;
/// Sidestep of walking agents, relative to the overlap, so that crossing agents pass each other.
const SIDESTEP: f32 = 0.6;

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

/// Body of a walking entity: agents keep `radius` away from each other.
#[derive(Component, Clone, Copy, Debug)]
pub struct NavAgent {
    pub radius: f32
}
impl Default for NavAgent {
    fn default() -> Self {
        Self {
            radius: DEFAULT_AGENT_RADIUS
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
        app.init_resource::<WalkSpeeds>()
            .init_resource::<WalkMap>()
            .add_systems(
                Update,
                (
                    walkmap::rebuild_walk_map.after(crate::navgrid::rebuild_nav_grid),
                    request_paths,
                    replan_on_network_change,
                    follow_paths,
                    separate_agents
                )
                    .chain()
            );
    }
}

/// Path followed by a moving entity.
#[derive(Component)]
pub(crate) struct NavPath {
    pub waypoints: VecDeque<Waypoint>,
    pub destination: Vec3,
    /// Remaining stop time at the waypoint just reached (airlock cycle).
    pub waiting: f32,
    /// Time spent unable to move toward the next waypoint.
    pub stuck: f32
}

/// Everything the path search reads.
#[derive(SystemParam)]
struct NavWorldParam<'w, 's> {
    grid: Res<'w, Grid>,
    placed: Query<'w, 's, (Entity, &'static GridEntity), With<BuildingConnections>>,
    walk: Res<'w, WalkMap>,
    network: Res<'w, NavGrid>
}
impl NavWorldParam<'_, '_> {
    fn world(&self) -> NavWorld<'_> {
        NavWorld {
            grid: &self.grid,
            buildings: self.placed.iter().collect::<HashMap<_, _>>(),
            walk: &self.walk,
            network: &self.network
        }
    }

    fn is_changed(&self) -> bool {
        self.walk.is_changed() || self.network.is_changed()
    }
}

/// Matches agents to spots, closest pairs first. Agents left without a spot get `None`.
fn assign_spots(agents: &[(Entity, Vec3)], spots: &[Vec3]) -> Vec<(Entity, Option<Vec3>)> {
    let mut pairs: Vec<(f32, usize, usize)> = agents
        .iter()
        .enumerate()
        .flat_map(|(a, (_, pos))| {
            spots
                .iter()
                .enumerate()
                .map(move |(s, spot)| (pos.xz().distance_squared(spot.xz()), a, s))
        })
        .collect();
    pairs.sort_unstable_by(|x, y| x.0.total_cmp(&y.0));
    let mut agent_spot = vec![None; agents.len()];
    let mut taken = vec![false; spots.len()];
    for (_, a, s) in pairs {
        if agent_spot[a].is_none() && !taken[s] {
            agent_spot[a] = Some(spots[s]);
            taken[s] = true;
        }
    }
    agents.iter().map(|(e, _)| *e).zip(agent_spot).collect()
}

/// Computes the paths requested through the [`TerrainNavigator`].
fn request_paths(
    mut commands: Commands,
    mut navigator: ResMut<TerrainNavigator>,
    transforms: Query<&Transform>,
    agents: Query<&NavAgent>,
    nav: NavWorldParam,
    speeds: Res<WalkSpeeds>
) {
    if navigator.new_entities.is_empty()
        && navigator.groups.is_empty()
        && navigator.removed_entities.is_empty()
    {
        return;
    }
    let world = nav.world();

    // Groups: spread their destination into one standing spot per entity
    let mut requests: Vec<(Entity, Vec3)> = navigator.new_entities.drain(..).collect();
    for (entities, destination) in std::mem::take(&mut navigator.groups) {
        let members: Vec<(Entity, Vec3)> = entities
            .iter()
            .filter_map(|e| transforms.get(*e).ok().map(|t| (*e, t.translation)))
            .collect();
        let radius = members
            .iter()
            .map(|(e, _)| agents.get(*e).map_or(DEFAULT_AGENT_RADIUS, |a| a.radius))
            .fold(0.0, f32::max);
        let spacing = 2.0 * radius * FORMATION_SPACING;
        let spots = formation_spots(&world, destination, members.len(), spacing);
        requests.extend(
            assign_spots(&members, &spots)
                .into_iter()
                .map(|(entity, spot)| (entity, spot.unwrap_or(destination)))
        );
    }

    for (entity, destination) in requests {
        let Ok(transform) = transforms.get(entity) else {
            continue;
        };
        match find_path(&world, &speeds, transform.translation, destination) {
            Some(waypoints) => {
                commands.entity(entity).insert(NavPath {
                    waypoints: waypoints.into(),
                    destination,
                    waiting: 0.0,
                    stuck: 0.0
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
    nav: NavWorldParam,
    mut moving: Query<(&Transform, &mut NavPath)>,
    speeds: Res<WalkSpeeds>
) {
    if !nav.is_changed() || moving.is_empty() {
        return;
    }
    let world = nav.world();
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
/// Where a step from `from` ends: the full step if it stays walkable, else its slide along the
/// blocking axis, else `None`. Off the walkable area (e.g. pushed), steps are free so that the
/// walker gets back on it.
fn walk_step(world: &NavWorld, from: Vec3, step: Vec3) -> Option<Vec3> {
    let Some(here) = world.locate(from.xz()) else {
        return Some(from + step);
    };
    [
        step,
        step * Vec3::new(1.0, 1.0, 0.0),
        step * Vec3::new(0.0, 1.0, 1.0)
    ]
    .into_iter()
    .filter(|s| s.xz().length_squared() > 1e-10)
    .map(|s| from + s)
    .find(|to| {
        world
            .locate(to.xz())
            .is_some_and(|there| world.can_cross(&here, &there))
    })
}

fn follow_paths(
    mut commands: Commands,
    mut moving: Query<(Entity, &mut Transform, &mut NavPath, Option<&Indoors>)>,
    nav: NavWorldParam,
    speeds: Res<WalkSpeeds>,
    time: Res<Time>
) {
    let dt = time.delta_secs();
    let world = nav.world();
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
            if waypoint.wait > 0.0 && distance <= AIRLOCK_REACHED {
                path.waypoints.pop_front();
                path.waiting = waypoint.wait;
                continue;
            }
            let reach = speed * time_left;
            if distance <= WAYPOINT_REACHED.max(reach) {
                transform.translation = waypoint.position;
                time_left -= distance / speed.max(1e-3);
                path.waypoints.pop_front();
                path.waiting = waypoint.wait;
                continue;
            }
            // Never through walls, even when pushed off the planned line
            match walk_step(&world, transform.translation, to_target / distance * reach) {
                Some(next) => {
                    transform.translation = next;
                    heading = Some(to_target);
                    path.stuck = 0.0;
                }
                None => path.stuck += time_left
            }
            time_left = 0.0;
        }
        if path.stuck > STUCK_REPLAN {
            path.stuck = 0.0;
            if let Some(waypoints) =
                find_path(&world, &speeds, transform.translation, path.destination)
            {
                path.waypoints = waypoints.into();
            }
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

/// Keeps agents from overlapping: overlapping agents are pushed apart, standing ones making way
/// for walking ones, and walking agents step aside to pass each other. Pushes never cross walls.
fn separate_agents(
    mut agents: Query<(&NavAgent, &mut Transform, Option<&NavPath>)>,
    nav: NavWorldParam,
    time: Res<Time>
) {
    let rate = (time.delta_secs() * SEPARATION_RATE).min(1.0);
    if rate <= 0.0 {
        return;
    }

    // (position, radius, mobility, heading)
    let bodies: Vec<(Vec2, f32, f32, Option<Vec2>)> = agents
        .iter()
        .map(|(agent, transform, path)| {
            let position = transform.translation.xz();
            let (mobility, heading) = match path {
                // Cycling an airlock: stays put
                Some(path) if path.waiting > 0.0 => (0.0, None),
                Some(path) => (
                    WALKER_YIELD,
                    path.waypoints
                        .front()
                        .and_then(|w| (w.position.xz() - position).try_normalize())
                ),
                None => (1.0, None)
            };
            (position, agent.radius, mobility, heading)
        })
        .collect();
    if bodies.len() < 2 {
        return;
    }

    // Spatial hash, cells as wide as the largest agent
    let cell = 2.0 * bodies.iter().map(|b| b.1).fold(0.0, f32::max).max(0.01);
    let key = |p: Vec2| (p / cell).floor().as_ivec2();
    let mut cells: HashMap<IVec2, Vec<usize>> = HashMap::new();
    for (i, body) in bodies.iter().enumerate() {
        cells.entry(key(body.0)).or_default().push(i);
    }

    let mut pushes = vec![Vec2::ZERO; bodies.len()];
    for (i, &(pi, ri, mi, hi)) in bodies.iter().enumerate() {
        let k = key(pi);
        for dx in -1..=1 {
            for dy in -1..=1 {
                for &j in cells.get(&(k + IVec2::new(dx, dy))).into_iter().flatten() {
                    if j <= i {
                        continue;
                    }
                    let (pj, rj, mj, hj) = bodies[j];
                    let overlap = ri + rj - pi.distance(pj);
                    if overlap <= 0.0 || mi + mj <= 0.0 {
                        continue;
                    }
                    // Agents on the same point: split them along an arbitrary, stable axis
                    let normal = (pi - pj)
                        .try_normalize()
                        .unwrap_or_else(|| Vec2::from_angle(i as f32 * 2.399));
                    pushes[i] += normal * overlap * mi / (mi + mj);
                    pushes[j] -= normal * overlap * mj / (mi + mj);
                    // Walkers keep to their right
                    if let Some(h) = hi {
                        pushes[i] += Vec2::new(-h.y, h.x) * overlap * SIDESTEP;
                    }
                    if let Some(h) = hj {
                        pushes[j] += Vec2::new(-h.y, h.x) * overlap * SIDESTEP;
                    }
                }
            }
        }
    }
    if pushes.iter().all(|p| *p == Vec2::ZERO) {
        return;
    }

    let world = nav.world();
    for ((_, mut transform, _), push) in agents.iter_mut().zip(pushes) {
        if push == Vec2::ZERO {
            continue;
        }
        let from = transform.translation.xz();
        let to = from + push * rate;
        // Never pushed through a wall or into an obstacle
        if let Some(here) = world.locate(from)
            && let Some(there) = world.locate(to)
            && world.can_cross(&here, &there)
        {
            transform.translation.x = to.x;
            transform.translation.z = to.y;
        }
    }
}
