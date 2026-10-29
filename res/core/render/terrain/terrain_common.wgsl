// Must match `TerrainDescription` in wde-terrain/src/render/dependencies/terrain_buffer.rs
struct TerrainDescription {
    tile_size:         vec3<f32>,            // (chunk size, height range, chunk size)
    tile_subdivisions: f32,                  // quads per chunk side at full detail
    min_height:        f32,                  // world height of a heightmap value of 0
    map_size:          f32,                  // texels per side of a chunk map (samples + border)
    debug_view:        f32,                  // 0 off, 1 buildable, 2 slope
    walk_max_angle:    f32,                  // slope angle of the steepest walkable ground, over 90°
    tiling:            array<vec4<f32>, 2>,  // world size of one texture repeat, per material
    height_blend:      array<vec4<f32>, 2>,  // relief-based blend sharpness, per material
}

// Must match `TerrainTileDescription`
struct TerrainTile {
    pos:           vec2<f32>,
    lod:           f32,
    tile_layer:    u32,
    neighbor_lods: vec4<f32>,  // -X, +X, -Z, +Z
}

const TERRAIN_MATERIALS: u32 = 8u;

fn material_value(values: array<vec4<f32>, 2>, material: u32) -> f32 {
    return values[material / 4u][material % 4u];
}
