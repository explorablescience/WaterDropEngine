//! A* pathfinding over the colony.
//!
//! The search graph mixes two layers:
//! - Outdoor grid tiles, walkable when empty or covered by a path piece (8 directions, no corner
//!   cutting past obstacles).
//! - The building network: each building is a node at its center linked to its anchors. Meeting
//!   tunnel anchors are the same node, so colonists walk through connected tunnels, domes and
//!   rooms. Path anchors (`AnchorOut`) open on the outdoor tile in front of them, so buildings are
//!   entered through their accesses and the tunnel network through airlocks.
//!
//! Each step costs its length divided by the walking speed factor of its terrain (see
//! [`WalkSpeeds`]), so colonists prefer paths and tunnels over bare ground.

use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap}
};

use bevy::prelude::*;
use wde_terrain_grid::{core::grid::TILE_SIZE, prelude::*};

use crate::navigation::WalkSpeeds;

/// Placement role of airlocks, where colonists stop to cycle between outside and inside.
pub const AIRLOCK_ROLE: &str = "airlock";

/// Building points are merged on a grid of this many cells per unit (tolerates anchors that are
/// slightly off, as in the older models).
const POINT_RESOLUTION: f32 = 4.0;
/// Extra space searched around the start and goal, in world units.
const SEARCH_MARGIN: f32 = 40.0;
/// Upper bound of expanded nodes, to keep unreachable targets cheap.
const MAX_EXPANSIONS: usize = 50_000;
/// Height of waypoints inside buildings (tunnel and dome floors).
const INDOOR_HEIGHT: f32 = 0.16;
/// Distance kept between standing spots and the walls of a building.
const WALL_MARGIN: f32 = 0.4;
/// Rings of candidate spots searched around a group destination.
const MAX_FORMATION_RINGS: usize = 24;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Node {
    /// Outdoor tile, by tile index.
    Tile(IVec2),
    /// Building center or anchor, by quantized position.
    Point(IVec2)
}

/// Kind of ground a node stands on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Terrain {
    /// Bare outdoor ground.
    Ground,
    /// Outdoor path, or the paved access of a building.
    Path,
    /// Pressurized building or tunnel.
    Indoor,
    /// Center of an airlock, between outside and inside.
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

    fn pressurized(self) -> bool {
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

/// Placed buildings and grid occupancy used to build the search graph.
pub(crate) struct NavWorld<'a> {
    pub grid: &'a Grid,
    pub buildings: HashMap<Entity, &'a GridEntity>
}

impl NavWorld<'_> {
    fn tile_key(pos: Vec2) -> IVec2 {
        (pos / TILE_SIZE).floor().as_ivec2()
    }

    fn tile_center(key: IVec2) -> Vec2 {
        Grid::get_tile_world_pos(Grid::get_nearest_tile((key.as_vec2() + 0.5) * TILE_SIZE))
    }

    fn point_key(pos: Vec2) -> IVec2 {
        (pos * POINT_RESOLUTION).round().as_ivec2()
    }

    fn occupant(&self, pos: Vec2) -> Option<&GridEntity> {
        let (chunk, local) = Grid::get_nearest_tile(pos);
        self.grid
            .get_entity(chunk, local)
            .and_then(|e| self.buildings.get(&e).copied())
    }

    /// Outdoor network pieces (paths) are walked on like free tiles.
    fn is_path_piece(building: &GridEntity) -> bool {
        let entry = building.entry();
        entry.network.is_some() && entry.anchors_in.is_empty()
    }

    /// Terrain of an outdoor tile, `None` when a building blocks it.
    fn tile_terrain(&self, pos: Vec2) -> Option<Terrain> {
        match self.occupant(pos) {
            None => Some(Terrain::Ground),
            Some(b) if Self::is_path_piece(b) => Some(Terrain::Path),
            Some(_) => None
        }
    }

    fn walkable(&self, pos: Vec2) -> bool {
        self.tile_terrain(pos).is_some()
    }

    /// Building (not a path piece) covering `pos`.
    fn building_at(&self, pos: Vec2) -> Option<&GridEntity> {
        self.occupant(pos).filter(|b| !Self::is_path_piece(b))
    }

    /// Whether `b` can be reached from `a` without crossing a wall: both in the same building, or
    /// both on walkable outdoor tiles.
    pub fn same_zone(&self, a: Vec2, b: Vec2) -> bool {
        match (self.building_at(a), self.building_at(b)) {
            (Some(x), Some(y)) => std::ptr::eq(x, y),
            (None, None) => self.walkable(b),
            _ => false
        }
    }
}

/// Up to `count` standing spots around `center`, `spacing` apart on a hexagonal pattern, nearest
/// first. Spots stay in the zone of `center` (its building, or outdoors), away from walls.
pub(crate) fn formation_spots(
    world: &NavWorld,
    center: Vec3,
    count: usize,
    spacing: f32
) -> Vec<Vec3> {
    let origin = center.xz();
    // Inside a building, keep to the circle inscribed in its footprint (domes are round)
    let building = world.building_at(origin).map(|b| {
        let (a, c) = b.bbox();
        let inscribed = ((a - c).abs() + TILE_SIZE).min_element() / 2.0 - WALL_MARGIN;
        (b.center(), inscribed.max(origin.distance(b.center())))
    });
    let valid = |p: Vec2| {
        world.same_zone(origin, p)
            && building.is_none_or(|(building_center, reach)| p.distance(building_center) <= reach)
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
        for p in candidates.into_iter().filter(|p| valid(*p)) {
            spots.push(Vec3::new(p.x, center.y, p.y));
            if spots.len() == count {
                return spots;
            }
        }
    }
    spots
}

/// Graph of the buildings: centers, anchors and the outdoor tiles their accesses open on.
#[derive(Default)]
struct BuildingGraph {
    points: HashMap<IVec2, (Vec2, Terrain)>,
    edges: HashMap<Node, Vec<Node>>
}
impl BuildingGraph {
    fn point(&mut self, pos: Vec2, terrain: Terrain) -> Node {
        let key = NavWorld::point_key(pos);
        let entry = self.points.entry(key).or_insert((pos, terrain));
        // A point shared with a pressurized building is pressurized
        if terrain.pressurized() && !entry.1.pressurized() {
            entry.1 = terrain;
        }
        Node::Point(key)
    }

    fn link(&mut self, a: Node, b: Node) {
        self.edges.entry(a).or_default().push(b);
        self.edges.entry(b).or_default().push(a);
    }

    fn build(world: &NavWorld, bounds: Rect) -> Self {
        let mut graph = Self::default();
        for building in world.buildings.values() {
            if NavWorld::is_path_piece(building) || !bounds.contains(building.center()) {
                continue;
            }
            let entry = building.entry();
            let pressurized = !entry.anchors_in.is_empty();
            let center_terrain = match (entry.role.as_deref(), pressurized) {
                (Some(AIRLOCK_ROLE), _) => Terrain::Airlock,
                (_, true) => Terrain::Indoor,
                (_, false) => Terrain::Path
            };
            let center = building.center();
            let center_node = graph.point(center, center_terrain);
            let (a, b) = building.bbox();
            let half = Vec2::splat(TILE_SIZE / 2.0);
            let (min, max) = (a.min(b) - half, a.max(b) + half);
            for anchor in &entry.anchors_in {
                let node = graph.point(building.anchor_world(anchor.pos), Terrain::Indoor);
                graph.link(center_node, node);
            }
            for anchor in &entry.anchors_out {
                // Outdoor accesses are paved (or the outer door of an airlock)
                let pos = building.anchor_world(anchor.pos);
                let node = graph.point(pos, Terrain::Path);
                graph.link(center_node, node);

                // The access opens on the tile in front of the side it is on
                let normal = if (pos.x - min.x).abs() < 0.2 {
                    Vec2::NEG_X
                } else if (pos.x - max.x).abs() < 0.2 {
                    Vec2::X
                } else if (pos.y - min.y).abs() < 0.2 {
                    Vec2::NEG_Y
                } else {
                    Vec2::Y
                };
                let outside = pos + normal * TILE_SIZE / 2.0;
                if world.walkable(outside) {
                    graph.link(node, Node::Tile(NavWorld::tile_key(outside)));
                }
            }
        }
        graph
    }
}

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
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap on f
        other.f.total_cmp(&self.f)
    }
}

/// Fastest walkable path from `start` to `goal` (start excluded, goal included).
pub(crate) fn find_path(
    world: &NavWorld,
    speeds: &WalkSpeeds,
    start: Vec3,
    goal: Vec3
) -> Option<Vec<Waypoint>> {
    let (start2, goal2) = (start.xz(), goal.xz());
    let bounds = Rect::from_corners(start2, goal2).inflate(SEARCH_MARGIN);
    let graph = BuildingGraph::build(world, bounds);

    // Start and goal: inside a building, they snap to its center; outdoors, to their tile
    let endpoint = |pos: Vec2| -> Option<Node> {
        if let Some(building) = world.building_at(pos) {
            let key = NavWorld::point_key(building.center());
            return graph.points.contains_key(&key).then_some(Node::Point(key));
        }
        world
            .walkable(pos)
            .then(|| Node::Tile(NavWorld::tile_key(pos)))
    };
    let (start_node, goal_node) = (endpoint(start2)?, endpoint(goal2)?);
    let describe = |node: Node| -> (Vec2, Terrain) {
        match node {
            Node::Tile(key) => {
                let center = NavWorld::tile_center(key);
                (
                    center,
                    world.tile_terrain(center).unwrap_or(Terrain::Ground)
                )
            }
            Node::Point(key) => graph.points[&key]
        }
    };
    let (goal_pos, goal_terrain) = describe(goal_node);

    // Same building or tile: walk straight there (e.g. across a dome)
    if start_node == goal_node {
        let height = match goal_node {
            Node::Tile(_) => 0.0,
            Node::Point(_) => INDOOR_HEIGHT
        };
        return Some(vec![Waypoint {
            position: Vec3::new(goal.x, height, goal.z),
            speed: goal_terrain.speed(speeds),
            indoor: goal_terrain.pressurized(),
            wait: 0.0
        }]);
    }
    // Admissible heuristic: straight line at the fastest speed
    let fastest = speeds.indoor.max(speeds.path).max(speeds.ground).max(0.01);
    let heuristic = |pos: Vec2| pos.distance(goal_pos) / fastest;

    // A*
    let mut open = BinaryHeap::from([Open {
        f: heuristic(describe(start_node).0),
        node: start_node
    }]);
    let mut came_from: HashMap<Node, Node> = HashMap::new();
    let mut cost: HashMap<Node, f32> = HashMap::from([(start_node, 0.0)]);
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

        neighbors.clear();
        if let Some(edges) = graph.edges.get(&node) {
            neighbors.extend_from_slice(edges);
        }
        if let Node::Tile(key) = node {
            let center = NavWorld::tile_center(key);
            let free = |dx: i32, dz: i32| {
                let pos = center + Vec2::new(dx as f32, dz as f32) * TILE_SIZE;
                bounds.contains(pos) && world.walkable(pos)
            };
            for dx in -1..=1 {
                for dz in -1..=1 {
                    if (dx, dz) == (0, 0) || !free(dx, dz) {
                        continue;
                    }
                    // No corner cutting past an obstacle
                    if dx != 0 && dz != 0 && !(free(dx, 0) && free(0, dz)) {
                        continue;
                    }
                    neighbors.push(Node::Tile(key + IVec2::new(dx, dz)));
                }
            }
        }

        let (pos, terrain) = describe(node);
        let node_cost = cost[&node];
        for &next in &neighbors {
            let (next_pos, next_terrain) = describe(next);
            let next_cost =
                node_cost + pos.distance(next_pos) / step_speed(terrain, next_terrain, speeds);
            if cost.get(&next).is_some_and(|c| *c <= next_cost) {
                continue;
            }
            cost.insert(next, next_cost);
            came_from.insert(next, node);
            open.push(Open {
                f: next_cost + heuristic(next_pos),
                node: next
            });
        }
    }
    if !cost.contains_key(&goal_node) {
        return None;
    }

    // Nodes from start to goal
    let mut nodes = vec![goal_node];
    while let Some(previous) = came_from.get(nodes.last().unwrap()) {
        nodes.push(*previous);
    }
    nodes.reverse();

    // Positions and terrains, with the exact start and goal positions at both ends
    let mut points: Vec<(Vec3, Terrain, bool)> = nodes
        .iter()
        .map(|node| {
            let (p, terrain) = describe(*node);
            let is_tile = matches!(node, Node::Tile(_));
            let height = if is_tile { 0.0 } else { INDOOR_HEIGHT };
            (Vec3::new(p.x, height, p.y), terrain, is_tile)
        })
        .collect();
    points[0].0 = start;
    let last = points.len() - 1;
    points[last].0 = Vec3::new(goal.x, points[last].0.y, goal.z);
    let points = smooth(world, &points);

    // Waypoints: speed and environment of the segment leading to each, airlock stops
    let mut waypoints = Vec::with_capacity(points.len());
    for i in 1..points.len() {
        let (from, to) = (points[i - 1], points[i]);
        let crossing = to.1 == Terrain::Airlock
            && i + 1 < points.len()
            && from.1.pressurized() != points[i + 1].1.pressurized();
        waypoints.push(Waypoint {
            position: to.0,
            speed: step_speed(from.1, to.1, speeds),
            indoor: from.1.pressurized() && to.1.pressurized(),
            wait: if crossing { speeds.airlock_cycle } else { 0.0 }
        });
    }
    Some(waypoints)
}

/// Removes outdoor waypoints that a straight line on the same terrain can skip.
fn smooth(world: &NavWorld, points: &[(Vec3, Terrain, bool)]) -> Vec<(Vec3, Terrain, bool)> {
    let mut result = vec![points[0]];
    let mut from = 0;
    for i in 1..points.len() {
        let can_skip = i + 1 < points.len()
            && points[from].2
            && points[i].2
            && points[i + 1].2
            && points[from].1 == points[i].1
            && points[i].1 == points[i + 1].1
            && line_on_terrain(
                world,
                points[from].0.xz(),
                points[i + 1].0.xz(),
                points[i].1
            );
        if !can_skip {
            result.push(points[i]);
            from = i;
        }
    }
    result
}

/// Whether the straight line from `a` to `b` stays on walkable tiles of the given terrain.
fn line_on_terrain(world: &NavWorld, a: Vec2, b: Vec2, terrain: Terrain) -> bool {
    let steps = (a.distance(b) / (TILE_SIZE * 0.25)).ceil() as usize;
    (0..=steps)
        .all(|i| world.tile_terrain(a.lerp(b, i as f32 / steps.max(1) as f32)) == Some(terrain))
}
