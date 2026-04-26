const MAX_SPLAT_LAYERS: u32 = 4u;

struct TerrainDescription {
    tile_size: vec3<f32>,
    tile_subdivisions: f32,
    displacement_scales: vec4<f32>,
    tiling_scales: vec4<f32>,
}

fn splat_weight(weights: vec4<f32>, layer: u32) -> f32 {
    if layer == 0u {
        return weights.x;
    }
    if layer == 1u {
        return weights.y;
    }
    if layer == 2u {
        return weights.z;
    }
    return 1.0 - weights.w;
}

fn terrain_layer_count(texture_layer_count: u32) -> u32 {
    return min(texture_layer_count, MAX_SPLAT_LAYERS);
}


fn get_terrain_uv(
    tex_coord: vec2<f32>,
    tile_pos: vec2<f32>,
    terrain: TerrainDescription,
    layer: u32
) -> vec2<f32> {
    let tiling_scale = terrain.tiling_scales[layer];
    return fract(tex_coord * tiling_scale + tile_pos * terrain.tile_size.xz * tiling_scale);
}
