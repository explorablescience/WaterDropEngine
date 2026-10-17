//! Walkable floor inside buildings, baked from their geometry.
//!
//! Each building model (its cutaway variant when it has one, which shows the interior) is sampled
//! once into a grid of cells: a cell is walkable when it has a floor and nothing between step and
//! head height above it. Parts shown only for some connection states (`%V%` variants, e.g. a
//! wall closing an unused doorway) are kept apart, so that placed buildings combine the parts
//! matching their connections. Doors (`%P%Door*` boxes) open for colonists: their geometry does
//! not block.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc
};

use bevy::prelude::*;
use wde_gltf::prelude::*;
use wde_terrain_grid::{core::grid::TILE_SIZE, prelude::*};

use crate::navigation::pathfinding::AIRLOCK_ROLE;

/// Side of a walk cell: tiles are split in 8 x 8 cells.
pub(crate) const CELL_SIZE: f32 = TILE_SIZE / 8.0;
/// Highest floor walked on (foundations, raised floors).
const FLOOR_MAX_HEIGHT: f32 = 0.5;
/// Surfaces at least this horizontal are floors.
const FLOOR_MIN_NORMAL_Y: f32 = 0.8;
/// Height above the floor that must be free of geometry: up to the shoulders of a colonist
/// (about 0.63 tall), the head passing under door frames and tunnel rings.
const HEAD_HEIGHT: f32 = 0.55;
/// Geometry less than this above the floor is stepped over (floor seams, door sills).
const STEP_HEIGHT: f32 = 0.1;
/// Distance between geometry samples, finer than the cells.
const SAMPLE_SPACING: f32 = CELL_SIZE / 3.0;
/// Cells whose center is closer than this to an obstacle are blocked too.
const OBSTACLE_RADIUS: f32 = 0.1;
/// Prefix of the `%P%` properties marking doors, as boxes (position, half extents).
const DOOR_PROPERTY_PREFIX: &str = "Door";
/// How far inside a building, from its doorways, its walkable region is searched.
const DOORWAY_DEPTH: f32 = 0.6;

/// Cells blocked by the parts sharing one connection condition.
struct Layer {
    condition: Option<ModelCondition>,
    blocked: Vec<bool>
}

/// Walkable cells of a building model, in its local space.
pub(crate) struct ModelWalkMap {
    size: UVec2,
    /// Floor height of each cell, `None` without floor.
    floor: Vec<Option<f32>>,
    layers: Vec<Layer>
}
impl ModelWalkMap {
    /// Samples the model triangles over its footprint. Without `ground_floor`, cells need a floor
    /// in the model; with it, the ground at height 0 is the floor (outdoor buildings).
    fn bake(asset: &GltfAsset, extent: UVec2, ground_floor: bool) -> Self {
        let size = extent * 8;
        let half = extent.as_vec2() * TILE_SIZE / 2.0;
        let cells = (size.x * size.y) as usize;
        let local_cell = |p: Vec2| ((p + half) / CELL_SIZE).floor().as_ivec2();
        let index = |c: IVec2| -> Option<usize> {
            (c.x >= 0 && c.y >= 0 && c.x < size.x as i32 && c.y < size.y as i32)
                .then(|| (c.y as u32 * size.x + c.x as u32) as usize)
        };
        let cell_of = |p: Vec3| index(local_cell(p.xz()));
        let doors: Vec<(Vec2, Vec2)> = asset
            .properties
            .iter()
            .filter(|(name, _)| name.starts_with(DOOR_PROPERTY_PREFIX))
            .map(|(_, (pos, scale))| (pos.xz(), scale.xz().abs()))
            .collect();
        let in_door = |p: Vec2| {
            doors
                .iter()
                .any(|(center, half)| (p - *center).abs().cmple(*half).all())
        };
        let samples = |geometry: &MeshGeometry, mut visit: Box<dyn FnMut(Vec3, bool) + '_>| {
            for tri in geometry.indices.chunks_exact(3) {
                let [a, b, c] = [0, 1, 2].map(|i| geometry.positions[tri[i] as usize]);
                let normal = (b - a).cross(c - a);
                let Some(normal) = normal.try_normalize() else {
                    continue;
                };
                let is_floor = normal.y >= FLOOR_MIN_NORMAL_Y;
                let longest = (b - a).length().max((c - a).length()).max((c - b).length());
                let n = (longest / SAMPLE_SPACING).ceil().max(1.0) as u32;
                for u in 0..=n {
                    for v in 0..=(n - u) {
                        let p =
                            a + (b - a) * (u as f32 / n as f32) + (c - a) * (v as f32 / n as f32);
                        visit(p, is_floor);
                    }
                }
            }
        };

        // Floors of every part, whatever its condition
        let mut floor: Vec<Option<f32>> = vec![ground_floor.then_some(0.0); cells];
        for geometry in &asset.geometry {
            samples(
                geometry,
                Box::new(|p, is_floor| {
                    if is_floor
                        && p.y <= FLOOR_MAX_HEIGHT
                        && let Some(c) = cell_of(p)
                    {
                        floor[c] = Some(floor[c].map_or(p.y, |f| f.max(p.y)));
                    }
                })
            );
        }

        // Obstacles between the floor and head height, per condition
        let mut layers: Vec<Layer> = Vec::new();
        for (geometry, condition) in asset.geometry.iter().zip(&asset.conditions) {
            let layer = match layers.iter().position(|l| l.condition == *condition) {
                Some(i) => i,
                None => {
                    layers.push(Layer {
                        condition: condition.clone(),
                        blocked: vec![false; cells]
                    });
                    layers.len() - 1
                }
            };
            let blocked = &mut layers[layer].blocked;
            samples(
                geometry,
                Box::new(|p, is_floor| {
                    let Some(c) = cell_of(p) else {
                        return;
                    };
                    let base = floor[c].unwrap_or(0.0);
                    let is_walked_floor = is_floor && p.y <= FLOOR_MAX_HEIGHT;
                    if is_walked_floor
                        || p.y <= base + STEP_HEIGHT
                        || p.y >= base + HEAD_HEIGHT
                        || in_door(p.xz())
                    {
                        return;
                    }
                    // Its cell, and the neighbors whose center is close
                    let cell = local_cell(p.xz());
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let n = cell + IVec2::new(dx, dy);
                            let center = (n.as_vec2() + 0.5) * CELL_SIZE - half;
                            if (n == cell || center.distance(p.xz()) <= OBSTACLE_RADIUS)
                                && let Some(i) = index(n)
                            {
                                blocked[i] = true;
                            }
                        }
                    }
                })
            );
        }
        Self {
            size,
            floor,
            layers
        }
    }
}

/// A walkable cell inside a building.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WalkCell {
    pub building: Entity,
    pub floor: f32
}

/// Walkable cells of all placed buildings (path pieces excepted, walked on like free tiles).
#[derive(Resource, Default)]
pub(crate) struct WalkMap {
    pub cells: HashMap<IVec2, WalkCell>,
    models: HashMap<AssetId<GltfAsset>, Arc<ModelWalkMap>>,
    /// Some models were not loaded yet at the last rebuild.
    incomplete: bool
}
impl WalkMap {
    pub fn cell_key(pos: Vec2) -> IVec2 {
        (pos / CELL_SIZE).floor().as_ivec2()
    }

    pub fn cell_center(key: IVec2) -> Vec2 {
        (key.as_vec2() + 0.5) * CELL_SIZE
    }
}

/// Whether a building is walked on like the ground around (outdoor network pieces).
pub(crate) fn is_path_piece(building: &GridEntity) -> bool {
    let entry = building.entry();
    entry.network.is_some() && entry.anchors_in.is_empty()
}

/// Whether a building is pressurized (it has tunnel anchors).
pub(crate) fn is_pressurized(building: &GridEntity) -> bool {
    !building.entry().anchors_in.is_empty()
}

pub(crate) fn is_airlock(building: &GridEntity) -> bool {
    building.entry().role.as_deref() == Some(AIRLOCK_ROLE)
}

/// Rebuilds the [`WalkMap`] when buildings are placed, removed or reconnected.
pub(crate) fn rebuild_walk_map(
    mut walk_map: ResMut<WalkMap>,
    changed: Query<(), Changed<BuildingConnections>>,
    mut removed: RemovedComponents<BuildingConnections>,
    placed: Query<(Entity, &GridEntity, &BuildingConnections)>,
    gltf_assets: Res<Assets<GltfAsset>>
) {
    let removed_any = removed.read().count() > 0;
    if changed.is_empty() && !removed_any && !walk_map.incomplete {
        return;
    }
    let walk_map = walk_map.as_mut();
    walk_map.cells.clear();
    walk_map.incomplete = false;
    for (entity, building, connections) in &placed {
        if is_path_piece(building) {
            continue;
        }
        // The cutaway variant shows the interior (the full one may be closed)
        let entry = building.entry();
        let handle = entry.asset_cut.as_ref().unwrap_or(&entry.asset);
        let model = match walk_map.models.get(&handle.id()) {
            Some(model) => model.clone(),
            None => {
                let Some(asset) = gltf_assets.get(handle) else {
                    walk_map.incomplete = true;
                    continue;
                };
                let model = Arc::new(ModelWalkMap::bake(
                    asset,
                    entry.extent,
                    !is_pressurized(building)
                ));
                walk_map.models.insert(handle.id(), model.clone());
                model
            }
        };
        add_building(&mut walk_map.cells, entity, building, connections, &model);
    }
}

/// Adds the walkable cells of a placed building: the parts matching its connections, away from
/// obstacles, and only the regions reachable from its doorways (or from outside).
fn add_building(
    cells: &mut HashMap<IVec2, WalkCell>,
    entity: Entity,
    building: &GridEntity,
    connections: &BuildingConnections,
    model: &ModelWalkMap
) {
    let size = model.size.as_ivec2();
    let index = |c: IVec2| (c.y * size.x + c.x) as usize;
    let inside = |c: IVec2| c.x >= 0 && c.y >= 0 && c.x < size.x && c.y < size.y;

    // Blocked by the parts shown for these connections, or without floor
    let mut blocked: Vec<bool> = model.floor.iter().map(Option::is_none).collect();
    for layer in &model.layers {
        let shown = layer
            .condition
            .as_ref()
            .is_none_or(|c| connections.0.contains(&c.anchor) == c.connected);
        if shown {
            for (b, layer_blocked) in blocked.iter_mut().zip(&layer.blocked) {
                *b |= *layer_blocked;
            }
        }
    }
    let walkable: Vec<bool> = blocked.iter().map(|b| !b).collect();

    // Local cell of a local position, and back
    let half = model.size.as_vec2() * CELL_SIZE / 2.0;
    let local_cell = |p: Vec2| ((p + half) / CELL_SIZE).floor().as_ivec2();
    let local_center = |c: IVec2| (c.as_vec2() + 0.5) * CELL_SIZE - half;

    // Seeds: just inside each doorway and the center for pressurized buildings, the border for
    // outdoor ones
    let mut seeds = Vec::new();
    if is_pressurized(building) {
        let entry = building.entry();
        let doorways = entry.anchors_in.iter().chain(match is_airlock(building) {
            true => entry.anchors_out.as_slice(),
            false => &[]
        });
        for anchor in doorways {
            let inward = (-anchor.pos).normalize_or_zero() * DOORWAY_DEPTH;
            seeds.push(local_cell(anchor.pos + inward));
        }
        seeds.push(local_cell(Vec2::ZERO));
    } else {
        for x in 0..size.x {
            seeds.extend([IVec2::new(x, 0), IVec2::new(x, size.y - 1)]);
        }
        for y in 0..size.y {
            seeds.extend([IVec2::new(0, y), IVec2::new(size.x - 1, y)]);
        }
    }

    // Flood fill the regions reachable from the seeds
    let mut reached = HashSet::new();
    let mut queue: VecDeque<IVec2> = seeds
        .into_iter()
        .filter(|c| inside(*c) && walkable[index(*c)])
        .collect();
    reached.extend(queue.iter().copied());
    while let Some(c) = queue.pop_front() {
        for d in [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y] {
            let n = c + d;
            if inside(n) && walkable[index(n)] && reached.insert(n) {
                queue.push_back(n);
            }
        }
    }

    for c in reached {
        let world = building.anchor_world(local_center(c));
        cells.insert(
            WalkMap::cell_key(world),
            WalkCell {
                building: entity,
                floor: model.floor[index(c)].unwrap_or(0.0)
            }
        );
    }
}
