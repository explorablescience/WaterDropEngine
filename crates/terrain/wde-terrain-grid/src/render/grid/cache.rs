use bevy::prelude::*;
use wde_terrain::prelude::CHUNK_SIZE;
use std::collections::HashMap;
use wde_gizmos::prelude::*;
use wde_renderer::prelude::*;

use crate::core::grid::{CHUNK_GRID_SUBDIVISIONS, GridChunkPos, GridLocalPos};

/// Marker component for grid gizmo entities.
#[derive(Component)]
pub struct GridGizmo;


/// Cache for grid rendering to avoid redundant asset creation and entity spawning.
#[derive(Resource, Default, Debug)]
pub struct GridGizmoCache {
    // Entities for each chunk's gizmos (grid lines, diagonal lines, occupied tile outlines).
    pub chunk_entities: HashMap<GridChunkPos, Vec<Entity>>,
    pub occupied_cell_entities: HashMap<(GridChunkPos, GridLocalPos), Entity>,

    // Meshes of each chunk
    pub chunk_grid_meshes: HashMap<GridChunkPos, Handle<MeshAsset>>,
    pub chunk_diagonal_meshes: HashMap<GridChunkPos, Handle<MeshAsset>>,
    pub occupied_cell_mesh: Option<Handle<MeshAsset>>,

    // Materials
    pub line_material: Option<Handle<GizmoMaterialAsset>>,
    pub diagonal_material: Option<Handle<GizmoMaterialAsset>>,
    pub occupied_cell_material: Option<Handle<GizmoMaterialAsset>>
}
impl GridGizmoCache {
    pub fn setup_materials_and_meshes(mut cache: ResMut<GridGizmoCache>, asset_server: Res<AssetServer>) {
        // Check if cache is already initialized to avoid redundant asset loading
        if cache.line_material.is_some() {
            return;
        }
        let cell_w = CHUNK_SIZE / CHUNK_GRID_SUBDIVISIONS as f32;
        let cell_d = CHUNK_SIZE / CHUNK_GRID_SUBDIVISIONS as f32;

        // Initialize materials and meshes
        cache.line_material = Some(asset_server.add(GizmoMaterialAsset {
            color: [0.05, 0.05, 0.05, 1.0],
            ..Default::default()
        }));
        cache.diagonal_material = Some(asset_server.add(GizmoMaterialAsset {
            color: [0.1, 0.1, 0.1, 1.0],
            ..Default::default()
        }));
        cache.occupied_cell_material = Some(asset_server.add(GizmoMaterialAsset {
            color: [0.2, 0.8, 0.2, 1.0],
            ..Default::default()
        }));
        cache.occupied_cell_mesh = Some(asset_server.add(Self::create_occupied_triangle_mesh(cell_w, cell_d)));
    }

    pub fn create_occupied_triangle_mesh(cell_w: f32, cell_d: f32) -> MeshAsset {
        // Smaller than a subtile triangle: 70% scale around subtile center.
        let sx = cell_w * 0.35;
        let sz = cell_d * 0.35;

        // Triangle centered at origin, pointing to +X/+Z (NorthEast in this grid convention).
        let vertices = vec![
            line_vertex(0.0, 0.0, -sz),
            line_vertex(sx, 0.0, 0.0),
            line_vertex(-sx, 0.0, 0.0),
        ];
        let indices = vec![0, 1, 1, 2, 2, 0];

        MeshAsset {
            label: "occupied_subtile_triangle".to_string(),
            vertices,
            indices,
            bounding_box: ModelBoundingBox {
                min: Vec3::new(-sx, 0.0, -sz),
                max: Vec3::new(sx, 0.0, sz),
            },
            use_ssbo: false,
        }
    }

    pub fn create_grid_lines_mesh(
        chunk_pos: GridChunkPos,
        chunk_min_x: f32,
        chunk_min_z: f32,
        cell_w: f32,
        cell_d: f32,
    ) -> MeshAsset {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for x_grid in 0..=CHUNK_GRID_SUBDIVISIONS {
            let x = chunk_min_x + (x_grid as f32) * cell_w;
            let z_start = chunk_min_z;
            let z_end = chunk_min_z + (CHUNK_GRID_SUBDIVISIONS as f32) * cell_d;

            let idx_start = vertices.len() as u32;
            vertices.push(line_vertex(x, 0.0, z_start));
            vertices.push(line_vertex(x, 0.0, z_end));
            indices.push(idx_start);
            indices.push(idx_start + 1);
        }

        for z_grid in 0..=CHUNK_GRID_SUBDIVISIONS {
            let z = chunk_min_z + (z_grid as f32) * cell_d;
            let x_start = chunk_min_x;
            let x_end = chunk_min_x + (CHUNK_GRID_SUBDIVISIONS as f32) * cell_w;

            let idx_start = vertices.len() as u32;
            vertices.push(line_vertex(x_start, 0.0, z));
            vertices.push(line_vertex(x_end, 0.0, z));
            indices.push(idx_start);
            indices.push(idx_start + 1);
        }

        MeshAsset {
            label: format!("grid_chunk_{chunk_pos:?}"),
            vertices,
            indices,
            bounding_box: chunk_bounds(chunk_min_x, chunk_min_z, cell_w, cell_d),
            use_ssbo: false,
        }
    }

    pub fn create_diagonal_lines_mesh(
        chunk_pos: GridChunkPos,
        chunk_min_x: f32,
        chunk_min_z: f32,
        cell_w: f32,
        cell_d: f32,
    ) -> MeshAsset {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for x_grid in 0..CHUNK_GRID_SUBDIVISIONS {
            for z_grid in 0..CHUNK_GRID_SUBDIVISIONS {
                let x0 = chunk_min_x + (x_grid as f32) * cell_w;
                let z0 = chunk_min_z + (z_grid as f32) * cell_d;
                let x1 = x0 + cell_w;
                let z1 = z0 + cell_d;

                let i0 = vertices.len() as u32;
                vertices.push(line_vertex(x0, 0.0, z0));
                vertices.push(line_vertex(x1, 0.0, z1));
                indices.push(i0);
                indices.push(i0 + 1);

                let i1 = vertices.len() as u32;
                vertices.push(line_vertex(x1, 0.0, z0));
                vertices.push(line_vertex(x0, 0.0, z1));
                indices.push(i1);
                indices.push(i1 + 1);
            }
        }

        MeshAsset {
            label: format!("diag_chunk_{chunk_pos:?}"),
            vertices,
            indices,
            bounding_box: chunk_bounds(chunk_min_x, chunk_min_z, cell_w, cell_d),
            use_ssbo: false,
        }
    }

}

/// Helper function to create a line vertex with given position and default normal/uv/tangent.
fn line_vertex(x: f32, y: f32, z: f32) -> Vertex {
    Vertex {
        position: [x, y, z],
        normal: [0.0, 1.0, 0.0],
        uv: [0.0, 0.0],
        tangent: [1.0, 0.0, 0.0, 0.0],
    }
}

/// Helper function to compute the bounding box of a chunk's grid/diagonal mesh based on its position and cell size.
fn chunk_bounds(chunk_min_x: f32, chunk_min_z: f32, cell_w: f32, cell_d: f32) -> ModelBoundingBox {
    ModelBoundingBox {
        min: Vec3::new(chunk_min_x, 0.0, chunk_min_z),
        max: Vec3::new(
            chunk_min_x + (CHUNK_GRID_SUBDIVISIONS as f32) * cell_w,
            0.0,
            chunk_min_z + (CHUNK_GRID_SUBDIVISIONS as f32) * cell_d,
        ),
    }
}
