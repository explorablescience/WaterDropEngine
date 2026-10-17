//! A* pathfinding over the colony.
//!
//! The search graph mixes two resolutions:
//! - Outdoor grid tiles, walkable when empty or covered by a path piece.
//! - Walk cells inside buildings (see [`WalkMap`]), baked from their geometry, so that colonists
//!   go around walls and furniture.
//!
//! Moves between two buildings follow tunnel links, and moves between pressurized and outdoor
//! places go through airlocks, where colonists stop for the airlock cycle. Each step costs its
//! length divided by the walking speed factor of its terrain (see [`WalkSpeeds`]), so colonists
//! prefer paths and tunnels over bare ground.

use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap}
};

use bevy::prelude::*;
use wde_terrain_grid::{core::grid::TILE_SIZE, prelude::*};

use crate::{
    navgrid::NavGrid,
    navigation::{
        WalkSpeeds,
        walkmap::{CELL_SIZE, WalkMap, is_airlock, is_path_piece, is_pressurized}
    }
};

/// Placement role of the buildings linking the outdoors to the tunnel network.
pub const AIRLOCK_ROLE: &str = "airlock";

/// Margin around the start and goal within which outdoor tiles are searched.
const SEARCH_MARGIN: f32 = 40.0;
/// Node expansions after which the search gives up.
const MAX_EXPANSIONS: usize = 80_000;
/// How far an unreachable start or goal is moved to the nearest walkable place.
const SNAP_DISTANCE: f32 = 3.0;
/// Step between the samples checking that a straight line stays walkable.
const LINE_STEP: f32 = CELL_SIZE * 0.5;
/// Half width kept walkable on both sides of a straight line, so that bodies do not brush walls.
const LINE_HALF_WIDTH: f32 = 0.08;
/// Rings of candidate spots searched around a group destination.
const MAX_FORMATION_RINGS: usize = 24;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Node {
    /// Outdoor tile (empty or path piece), by tile index.
    Tile(IVec2),
    /// Walk cell inside a building, by cell index.
    Cell(IVec2)
}

/// Kind of ground a node stands on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Terrain {
    /// Bare outdoor ground, or an outdoor building.
    Ground,
    /// Outdoor path.
    Path,
    /// Pressurized building or tunnel.
    Indoor,
    /// Airlock, between outside and inside.
    Airlock
}
impl Terrain {
    fn speed(self, speeds: &WalkSpeeds) -> f32 {
        match self {
            Terrain::Ground => speeds.ground,
            Terrain::Path => speeds.path,
            Terrain::Indoor | Terrain::Airlock => speeds.indoor
        }
    }

    pub fn pressurized(self) -> bool {
        matches!(self, Terrain::Indoor | Terrain::Airlock)
    }
}

/// Speed factor of the step between two nodes: the slower of both ends.
fn step_speed(a: Terrain, b: Terrain, speeds: &WalkSpeeds) -> f32 {
    a.speed(speeds).min(b.speed(speeds)).max(0.01)
}

/// Point of a path: where to go, how fast to get there, and what to do on arrival.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Waypoint {
    pub position: Vec3,
    /// Walking speed factor of the segment leading to this waypoint.
    pub speed: f32,
    /// Whether the segment leading to this waypoint is pressurized (helmet off).
    pub indoor: bool,
    /// Time to stand still once reached (airlock cycle), in seconds.
    pub wait: f32
}

/// Walkable place.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Location {
    /// Building walked in, `None` outdoors (path pieces included).
    pub building: Option<Entity>,
    pub terrain: Terrain,
    /// Floor height.
    pub height: f32
}

/// Placed buildings, grid occupancy and walk cells used to build the search graph.
pub(crate) struct NavWorld<'a> {
    pub grid: &'a Grid,
    pub buildings: HashMap<Entity, &'a GridEntity>,
    pub walk: &'a WalkMap,
    pub network: &'a NavGrid
}

impl NavWorld<'_> {
    fn tile_key(pos: Vec2) -> IVec2 {
        (pos / TILE_SIZE).floor().as_ivec2()
    }

    fn tile_center(key: IVec2) -> Vec2 {
        (key.as_vec2() + 0.5) * TILE_SIZE
    }

    fn occupant(&self, pos: Vec2) -> Option<(Entity, &GridEntity)> {
        let (chunk, local) = Grid::get_nearest_tile(pos);
        let entity = self.grid.get_entity(chunk, local)?;
        self.buildings.get(&entity).map(|b| (entity, *b))
    }

    /// Whether the tile at `pos` is walked as a whole (empty or path piece), rather than through
    /// the walk cells of a building.
    fn is_open_tile(&self, pos: Vec2) -> bool {
        self.occupant(pos).is_none_or(|(_, b)| is_path_piece(b))
    }

    /// Walkable place at `pos`, `None` if blocked.
    pub fn locate(&self, pos: Vec2) -> Option<Location> {
        match self.occupant(pos) {
            None => Some(Location {
                building: None,
                terrain: Terrain::Ground,
                height: 0.0
            }),
            Some((_, b)) if is_path_piece(b) => Some(Location {
                building: None,
                terrain: Terrain::Path,
                height: 0.0
            }),
            Some((entity, b)) => {
                let cell = self.walk.cells.get(&WalkMap::cell_key(pos))?;
                (cell.building == entity).then(|| Location {
                    building: Some(entity),
                    terrain: match (is_airlock(b), is_pressurized(b)) {
                        (true, _) => Terrain::Airlock,
                        (false, true) => Terrain::Indoor,
                        (false, false) => Terrain::Ground
                    },
                    height: cell.floor
                })
            }
        }
    }

    /// Whether one can step from `a` to the neighboring place `b`: within a building or the
    /// outdoors, between buildings linked by a tunnel, or through an airlock door.
    pub fn can_cross(&self, a: &Location, b: &Location) -> bool {
        if a.building == b.building {
            return true;
        }
        match (a.terrain.pressurized(), b.terrain.pressurized()) {
            (false, false) => true,
            (true, true) => match (a.building, b.building) {
                (Some(x), Some(y)) => self.network.buildings.get(&x).is_some_and(|building| {
                    building
                        .neighbors
                        .iter()
                        .any(|n| n.entity == y && n.kind == LinkKind::Tunnel)
                }),
                _ => false
            },
            _ => a.terrain == Terrain::Airlock || b.terrain == Terrain::Airlock
        }
    }

    /// Whether the straight line from `a` to `b` can be walked, with some width, on the same
    /// terrain all along.
    fn line_walkable(&self, a: Vec2, b: Vec2) -> bool {
        let side = (b - a).perp().normalize_or_zero() * LINE_HALF_WIDTH;
        [Vec2::ZERO, side, -side]
            .into_iter()
            .all(|offset| self.strip_walkable(a + offset, b + offset))
    }

    fn strip_walkable(&self, a: Vec2, b: Vec2) -> bool {
        let Some(mut previous) = self.locate(a) else {
            return false;
        };
        let steps = (a.distance(b) / LINE_STEP).ceil().max(1.0) as usize;
        for i in 1..=steps {
            let Some(here) = self.locate(a.lerp(b, i as f32 / steps as f32)) else {
                return false;
            };
            if here.terrain != previous.terrain || !self.can_cross(&previous, &here) {
                return false;
            }
            previous = here;
        }
        true
    }

    /// Nearest walkable place to `pos` within `max_distance`, accepted by `filter`.
    fn nearest_walkable(
        &self,
        pos: Vec2,
        max_distance: f32,
        filter: impl Fn(&Location) -> bool
    ) -> Option<(Vec2, Location)> {
        if let Some(here) = self.locate(pos).filter(&filter) {
            return Some((pos, here));
        }
        let rings = (max_distance / CELL_SIZE).ceil() as i32;
        let origin = WalkMap::cell_key(pos);
        let mut best: Option<(f32, Vec2, Location)> = None;
        for ring in 1..=rings {
            for dy in -ring..=ring {
                for dx in -ring..=ring {
                    if dx.abs() != ring && dy.abs() != ring {
                        continue;
                    }
                    let p = WalkMap::cell_center(origin + IVec2::new(dx, dy));
                    let distance = p.distance(pos);
                    if distance > max_distance || best.is_some_and(|b| b.0 <= distance) {
                        continue;
                    }
                    if let Some(here) = self.locate(p).filter(&filter) {
                        best = Some((distance, p, here));
                    }
                }
            }
            // Later rings are at least `ring` cells away
            if best.is_some_and(|b| b.0 <= ring as f32 * CELL_SIZE) {
                break;
            }
        }
        best.map(|(_, p, l)| (p, l))
    }

    /// Search node at a walkable position.
    fn node_at(&self, pos: Vec2) -> Node {
        match self.is_open_tile(pos) {
            true => Node::Tile(Self::tile_key(pos)),
            false => Node::Cell(WalkMap::cell_key(pos))
        }
    }

    fn node_position(node: Node) -> Vec2 {
        match node {
            Node::Tile(key) => Self::tile_center(key),
            Node::Cell(key) => WalkMap::cell_center(key)
        }
    }

    /// Walkable neighbors of a node, with their location.
    fn neighbors(&self, node: Node, from: &Location, out: &mut Vec<(Node, Location)>) {
        let push = |pos: Vec2, out: &mut Vec<(Node, Location)>| {
            if let Some(here) = self.locate(pos)
                && self.can_cross(from, &here)
            {
                let next = self.node_at(pos);
                if next != node && !out.iter().any(|(n, _)| *n == next) {
                    out.push((next, here));
                }
            }
        };
        match node {
            Node::Tile(key) => {
                let center = Self::tile_center(key);
                for d in DIRECTIONS {
                    let next = Self::tile_center(key + d);
                    let diagonal = d.x != 0 && d.y != 0;
                    if self.is_open_tile(next) {
                        // No corner cutting past a building
                        if !diagonal
                            || (self.is_open_tile(Self::tile_center(key + IVec2::new(d.x, 0)))
                                && self.is_open_tile(Self::tile_center(key + IVec2::new(0, d.y))))
                        {
                            push(next, out);
                        }
                    } else if !diagonal {
                        // Cells of the building along the shared side
                        let side = d.as_vec2() * (TILE_SIZE + CELL_SIZE) / 2.0;
                        let along = Vec2::new(d.y as f32, d.x as f32).abs();
                        for i in 0..8 {
                            let offset = (i as f32 - 3.5) * CELL_SIZE;
                            push(center + side + along * offset, out);
                        }
                    }
                }
            }
            Node::Cell(key) => {
                for d in DIRECTIONS {
                    // No corner cutting past an obstacle
                    let diagonal = d.x != 0 && d.y != 0;
                    if diagonal
                        && (self
                            .locate(WalkMap::cell_center(key + IVec2::new(d.x, 0)))
                            .is_none()
                            || self
                                .locate(WalkMap::cell_center(key + IVec2::new(0, d.y)))
                                .is_none())
                    {
                        continue;
                    }
                    push(WalkMap::cell_center(key + d), out);
                }
            }
        }
    }
}

const DIRECTIONS: [IVec2; 8] = [
    IVec2::new(1, 0),
    IVec2::new(-1, 0),
    IVec2::new(0, 1),
    IVec2::new(0, -1),
    IVec2::new(1, 1),
    IVec2::new(1, -1),
    IVec2::new(-1, 1),
    IVec2::new(-1, -1)
];

#[derive(PartialEq)]
struct Open {
    f: f32,
    node: Node
}
impl Eq for Open {}
impl PartialOrd for Open {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Open {
    // Min-heap on f
    fn cmp(&self, other: &Self) -> Ordering {
        other.f.total_cmp(&self.f)
    }
}

/// Point of a path before it becomes waypoints.
#[derive(Clone, Copy)]
struct PathPoint {
    position: Vec3,
    terrain: Terrain,
    /// Whether the colonist is on the pressurized side here (airlocks are both).
    pressurized: bool,
    /// Airlock cycle stop: kept by the smoothing.
    wait: f32
}

/// Path from `start` to `goal`, through walkable places only. An unreachable goal is moved to
/// the nearest walkable place.
pub(crate) fn find_path(
    world: &NavWorld,
    speeds: &WalkSpeeds,
    start: Vec3,
    goal: Vec3
) -> Option<Vec<Waypoint>> {
    let (start, start_location) = world.nearest_walkable(start.xz(), SNAP_DISTANCE, |_| true)?;
    let (goal, goal_location) = world.nearest_walkable(goal.xz(), SNAP_DISTANCE, |_| true)?;
    let (start_node, goal_node) = (world.node_at(start), world.node_at(goal));
    let bounds = Rect::from_corners(start, goal).inflate(SEARCH_MARGIN);

    // Admissible heuristic: straight line at the fastest speed
    let fastest = speeds.indoor.max(speeds.path).max(speeds.ground).max(0.01);
    let goal_position = NavWorld::node_position(goal_node);
    let heuristic = |pos: Vec2| pos.distance(goal_position) / fastest;

    // A*
    let mut open = BinaryHeap::from([Open {
        f: heuristic(start),
        node: start_node
    }]);
    let mut came_from: HashMap<Node, Node> = HashMap::new();
    let mut cost: HashMap<Node, f32> = HashMap::from([(start_node, 0.0)]);
    let mut locations: HashMap<Node, Location> =
        HashMap::from([(start_node, start_location), (goal_node, goal_location)]);
    let mut expansions = 0;
    let mut neighbors = Vec::with_capacity(16);
    while let Some(Open { node, .. }) = open.pop() {
        if node == goal_node {
            break;
        }
        expansions += 1;
        if expansions > MAX_EXPANSIONS {
            return None;
        }
        let location = locations[&node];
        let position = NavWorld::node_position(node);
        neighbors.clear();
        world.neighbors(node, &location, &mut neighbors);
        let node_cost = cost[&node];
        for &(next, next_location) in &neighbors {
            let next_position = NavWorld::node_position(next);
            if !bounds.contains(next_position) {
                continue;
            }
            let next_cost = node_cost
                + position.distance(next_position)
                    / step_speed(location.terrain, next_location.terrain, speeds);
            if cost.get(&next).is_some_and(|c| *c <= next_cost) {
                continue;
            }
            cost.insert(next, next_cost);
            came_from.insert(next, node);
            locations.insert(next, next_location);
            open.push(Open {
                f: next_cost + heuristic(next_position),
                node: next
            });
        }
    }
    if !came_from.contains_key(&goal_node) && start_node != goal_node {
        return None;
    }

    // Nodes from start to goal, with the exact start and goal positions at both ends
    let mut nodes = vec![goal_node];
    while let Some(previous) = came_from.get(nodes.last().unwrap()) {
        nodes.push(*previous);
    }
    nodes.reverse();
    let mut points: Vec<(Vec2, Location)> = nodes
        .iter()
        .map(|node| (NavWorld::node_position(*node), locations[node]))
        .collect();
    points[0] = (start, start_location);
    if points.len() == 1 {
        points.push((goal, goal_location));
    }
    let last = points.len() - 1;
    points[last] = (goal, goal_location);

    let points = airlock_stops(world, &points, speeds);
    let points = smooth(world, &points);

    // Waypoints: speed and environment of the segment leading to each
    Some(
        points
            .windows(2)
            .map(|pair| {
                let (from, to) = (pair[0], pair[1]);
                Waypoint {
                    position: to.position,
                    speed: step_speed(from.terrain, to.terrain, speeds),
                    indoor: from.pressurized && to.pressurized,
                    wait: to.wait
                }
            })
            .collect()
    )
}

/// Turns located points into path points, adding a stop at the center of each airlock crossed
/// from one side to the other.
fn airlock_stops(
    world: &NavWorld,
    points: &[(Vec2, Location)],
    speeds: &WalkSpeeds
) -> Vec<PathPoint> {
    let point = |(p, l): (Vec2, Location), pressurized: bool, wait: f32| PathPoint {
        position: Vec3::new(p.x, l.height, p.y),
        terrain: l.terrain,
        pressurized,
        wait
    };
    let mut result = Vec::with_capacity(points.len() + 2);
    let mut i = 0;
    while i < points.len() {
        if points[i].1.terrain != Terrain::Airlock {
            result.push(point(points[i], points[i].1.terrain.pressurized(), 0.0));
            i += 1;
            continue;
        }
        // Run of airlock points, and the sides it joins
        let end = (i..points.len())
            .find(|&j| points[j].1.terrain != Terrain::Airlock)
            .unwrap_or(points.len());
        let before = i.checked_sub(1).map(|j| points[j].1.terrain.pressurized());
        let after = points.get(end).map(|p| p.1.terrain.pressurized());
        let side_in = before.or(after).unwrap_or(true);
        let side_out = after.or(before).unwrap_or(true);
        let center = (side_in != side_out)
            .then(|| {
                let airlock = points[i].1.building?;
                let center = world.buildings.get(&airlock)?.center();
                world.nearest_walkable(center, TILE_SIZE, |l| l.building == Some(airlock))
            })
            .flatten();
        match center {
            Some(center) => {
                result.push(point(points[i], side_in, 0.0));
                result.push(point(center, side_in, speeds.airlock_cycle));
                result.push(point(points[end - 1], side_out, 0.0));
            }
            None => result.extend(points[i..end].iter().map(|p| point(*p, side_in, 0.0)))
        }
        i = end;
    }
    result
}

/// Skips the points that a straight walkable line on the same terrain can cut, keeping the
/// airlock stops.
fn smooth(world: &NavWorld, points: &[PathPoint]) -> Vec<PathPoint> {
    let mut result = vec![points[0]];
    let mut from = 0;
    while from + 1 < points.len() {
        let mut to = from + 1;
        while to + 1 < points.len()
            && points[to].wait <= 0.0
            && points[to + 1].pressurized == points[from].pressurized
            && world.line_walkable(points[from].position.xz(), points[to + 1].position.xz())
        {
            to += 1;
        }
        result.push(points[to]);
        from = to;
    }
    result
}

/// Up to `count` standing spots around `center`, `spacing` apart on a hexagonal pattern, nearest
/// first. Spots stay walkable and in the place of `center` (its building, or outdoors).
pub(crate) fn formation_spots(
    world: &NavWorld,
    center: Vec3,
    count: usize,
    spacing: f32
) -> Vec<Vec3> {
    let Some((origin, place)) = world.nearest_walkable(center.xz(), SNAP_DISTANCE, |_| true) else {
        return Vec::new();
    };
    let axes: Vec<Vec2> = (0..6)
        .map(|i| Vec2::from_angle(i as f32 * std::f32::consts::TAU / 6.0) * spacing)
        .collect();
    let mut spots = Vec::with_capacity(count);
    for ring in 0..=MAX_FORMATION_RINGS {
        let candidates: Vec<Vec2> = match ring {
            0 => vec![origin],
            _ => (0..6)
                .flat_map(|side| {
                    let corner = origin + axes[side] * ring as f32;
                    let step = axes[(side + 2) % 6];
                    (0..ring).map(move |i| corner + step * i as f32)
                })
                .collect()
        };
        for p in candidates {
            if let Some(here) = world.locate(p)
                && here.building == place.building
                && here.terrain.pressurized() == place.terrain.pressurized()
            {
                spots.push(Vec3::new(p.x, here.height, p.y));
                if spots.len() == count {
                    return spots;
                }
            }
        }
    }
    spots
}
