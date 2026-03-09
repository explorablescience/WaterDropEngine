use bevy::prelude::*;
use std::collections::HashMap;
use wde_terrain::prelude::CHUNK_SIZE;
use wde_gizmos::prelude::*;
use wde_renderer::prelude::*;

use crate::core::grid::{Grid, GridChunkPos, CHUNK_GRID_SUBDIVISIONS};

/// Component to track which gizmos belong to the grid visualization
#[derive(Component)]
pub struct GridGizmo;

/// Cache for grid visualization meshes and entities to avoid recreating them every frame.
#[derive(Resource, Default, Debug)]
pub struct GridGizmoCache {
    /// Cached mesh handles for each chunk's grid lines
    chunk_line_meshes: HashMap<GridChunkPos, Handle<MeshAsset>>,
    /// Cached entities for each chunk's grid visualization
    chunk_entities: HashMap<GridChunkPos, Vec<Entity>>,
    /// Cached material handles
    line_material: Option<Handle<GizmoMaterialAsset>>,
    occupied_material: Option<Handle<GizmoMaterialAsset>>,
}

/// Initializes the grid gizmo cache
pub fn init_grid_cache(mut commands: Commands) {
    commands.insert_resource(GridGizmoCache::default());
}

/// Creates grid line meshes and spawns them as gizmos.
/// This visualizes the grid structure by drawing:
/// - Lines at each tile boundary within chunks
/// - Colored squares at occupied tile positions
///
/// Meshes and materials are cached to avoid recreation every frame.
pub fn render_grid(
    mut commands: Commands,
    grid: Res<Grid>,
    assets: Res<AssetServer>,
    mut cache: ResMut<GridGizmoCache>,
) {
    // Initialize materials if not already cached
    if cache.line_material.is_none() {
        cache.line_material = Some(assets.add(GizmoMaterialAsset {
            color: [0.5, 0.5, 0.7, 1.0], // Light blue for grid lines
            ..Default::default()
        }));
    }
    if cache.occupied_material.is_none() {
        cache.occupied_material = Some(assets.add(GizmoMaterialAsset {
            color: [0.2, 0.8, 0.2, 0.8], // Green for occupied tiles
            ..Default::default()
        }));
    }

    let line_material = cache.line_material.clone().unwrap();
    let occupied_material = cache.occupied_material.clone().unwrap();

    // Get cell dimensions
    let cell_w = CHUNK_SIZE[0] / CHUNK_GRID_SUBDIVISIONS as f32;
    let cell_d = CHUNK_SIZE[2] / CHUNK_GRID_SUBDIVISIONS as f32;

    // Iterate through all chunks and create/update visualization
    for (&chunk_pos, chunk_data) in grid.get_chunks() {
        // Check if this chunk is already cached
        if cache.chunk_entities.contains_key(&chunk_pos) {
            continue; // Already rendered, skip
        }

        // Calculate chunk world position (chunk positions are centered)
        let chunk_min_x = (chunk_pos.x as f32 * CHUNK_SIZE[0]) - (CHUNK_SIZE[0] * 0.5);
        let chunk_min_z = (chunk_pos.y as f32 * CHUNK_SIZE[2]) - (CHUNK_SIZE[2] * 0.5);

        let mut chunk_gizmo_entities = Vec::new();

        // Create grid lines for this chunk (cached)
        let grid_mesh = if let Some(mesh_handle) = cache.chunk_line_meshes.get(&chunk_pos).cloned() {
            mesh_handle
        } else {
            let mesh = create_grid_lines_mesh(chunk_pos, chunk_min_x, chunk_min_z, cell_w, cell_d);
            let handle = assets.add(mesh);
            cache.chunk_line_meshes.insert(chunk_pos, handle.clone());
            handle
        };

        let grid_entity = commands.spawn((
            Mesh(grid_mesh),
            GizmoMaterial(line_material.clone()),
            Transform::IDENTITY,
            GridGizmo,
        )).id();
        chunk_gizmo_entities.push(grid_entity);

        // Render occupied tiles
        for (tile_idx, &tile_entity) in chunk_data.get_tiles().iter().enumerate() {
            if tile_entity.is_some() {
                // Calculate tile position from index
                let _dir_offset = tile_idx % 4;
                let tile_pos_idx = tile_idx / 4;
                let local_z = (tile_pos_idx / CHUNK_GRID_SUBDIVISIONS as usize) as u32;
                let local_x = (tile_pos_idx % CHUNK_GRID_SUBDIVISIONS as usize) as u32;

                // Calculate world position of the tile center
                let tile_x = chunk_min_x + (local_x as f32 + 0.5) * cell_w;
                let tile_z = chunk_min_z + (local_z as f32 + 0.5) * cell_d;

                // Create a small square mesh for the occupied tile
                let tile_size = Vec3::new(cell_w * 0.8, 0.1, cell_d * 0.8);
                let tile_mesh = CubeGizmoMesh::from(&format!("tile_{}", tile_idx), tile_size);
                let mesh_handle = assets.add(tile_mesh);

                let tile_gizmo_entity = commands.spawn((
                    Mesh(mesh_handle),
                    GizmoMaterial(occupied_material.clone()),
                    Transform::from_xyz(tile_x, 0.1, tile_z),
                    GridGizmo,
                )).id();
                chunk_gizmo_entities.push(tile_gizmo_entity);
            }
        }

        // Cache the entities for this chunk
        cache.chunk_entities.insert(chunk_pos, chunk_gizmo_entities);
    }
}

/// Creates a line mesh for the grid structure of a chunk.
/// Draws vertical and horizontal lines at tile boundaries.
fn create_grid_lines_mesh(
    chunk_pos: GridChunkPos,
    chunk_min_x: f32,
    chunk_min_z: f32,
    cell_w: f32,
    cell_d: f32,
) -> MeshAsset {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // We'll create a grid of lines
    // Horizontal lines (along Z axis) at each X grid line
    for x_grid in 0..=CHUNK_GRID_SUBDIVISIONS {
        let x = chunk_min_x + (x_grid as f32) * cell_w;
        
        let z_start = chunk_min_z;
        let z_end = chunk_min_z + (CHUNK_GRID_SUBDIVISIONS as f32) * cell_d;

        let idx_start = vertices.len() as u32;
        vertices.push(Vertex {
            position: [x, 0.0, z_start],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
            tangent: [1.0, 0.0, 0.0, 0.0],
        });
        vertices.push(Vertex {
            position: [x, 0.0, z_end],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 1.0],
            tangent: [1.0, 0.0, 0.0, 0.0],
        });

        indices.push(idx_start);
        indices.push(idx_start + 1);
    }

    // Vertical lines (along X axis) at each Z grid line
    for z_grid in 0..=CHUNK_GRID_SUBDIVISIONS {
        let z = chunk_min_z + (z_grid as f32) * cell_d;
        
        let x_start = chunk_min_x;
        let x_end = chunk_min_x + (CHUNK_GRID_SUBDIVISIONS as f32) * cell_w;

        let idx_start = vertices.len() as u32;
        vertices.push(Vertex {
            position: [x_start, 0.0, z],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
            tangent: [1.0, 0.0, 0.0, 0.0],
        });
        vertices.push(Vertex {
            position: [x_end, 0.0, z],
            normal: [0.0, 1.0, 0.0],
            uv: [1.0, 0.0],
            tangent: [1.0, 0.0, 0.0, 0.0],
        });

        indices.push(idx_start);
        indices.push(idx_start + 1);
    }

    let bounding_box = ModelBoundingBox {
        min: Vec3::new(chunk_min_x, 0.0, chunk_min_z),
        max: Vec3::new(
            chunk_min_x + (CHUNK_GRID_SUBDIVISIONS as f32) * cell_w,
            0.0,
            chunk_min_z + (CHUNK_GRID_SUBDIVISIONS as f32) * cell_d,
        ),
    };

    MeshAsset {
        label: format!("grid_chunk_{:?}", chunk_pos),
        vertices,
        indices,
        bounding_box,
        use_ssbo: false,
    }
}
