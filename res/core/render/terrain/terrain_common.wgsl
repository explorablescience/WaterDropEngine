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

// Returns raw (non-fractional) tiled UV; repeat wrapping is handled by the sampler.
fn terrain_layer_uv(tex_coord: vec2<f32>, terrain: TerrainDescription, layer: u32) -> vec2<f32> {
    return tex_coord * terrain.tiling_scales[layer];
}

fn sum3(v: vec3<f32>) -> f32 {
    return v.x + v.y + v.z;
}

// 2D -> 1D hash, output in [0, 1].
fn terrain_hash(p: vec2<f32>) -> f32 {
    var h = fract(p * vec2<f32>(0.1031, 0.1030));
    h += dot(h, h.yx + 33.33);
    return fract((h.x + h.y) * h.x);
}

// Smooth bicubic value noise in [0, 1].
fn terrain_smooth_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(terrain_hash(i + vec2<f32>(0.0, 0.0)), terrain_hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(terrain_hash(i + vec2<f32>(0.0, 1.0)), terrain_hash(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y
    );
}

fn terrain_texture_no_tile_rgb(
    source_tex: texture_2d_array<f32>,
    source_sampler: sampler,
    uv: vec2<f32>,    // Raw (non-fractional) world-continuous UV
    layer: u32,
    variation: f32
) -> vec3<f32> {
    // Derivatives of the raw UV are continuous (no fract discontinuities), giving correct mip levels.
    let duvdx = dpdx(uv);
    let duvdy = dpdy(uv);

    // Smooth stochastic cell index; scale 0.4 yields cells ~2-3 texture repeats wide.
    let k = terrain_smooth_noise(uv * 0.4);

    let l = k * 8.0;
    let f = fract(l);
    let ia = floor(l);
    let ib = ia + 1.0;

    let offa = sin(vec2<f32>(3.0, 7.0) * ia);
    let offb = sin(vec2<f32>(3.0, 7.0) * ib);

    let cola = textureSampleGrad(source_tex, source_sampler, fract(uv + variation * offa), layer, duvdx, duvdy).rgb;
    let colb = textureSampleGrad(source_tex, source_sampler, fract(uv + variation * offb), layer, duvdx, duvdy).rgb;

    let blend = smoothstep(0.2, 0.8, f - 0.1 * sum3(cola - colb));
    return mix(cola, colb, blend);
}

fn terrain_texture_no_tile_r(
    source_tex: texture_2d_array<f32>,
    source_sampler: sampler,
    uv: vec2<f32>,
    layer: u32,
    variation: f32
) -> f32 {
    return terrain_texture_no_tile_rgb(source_tex, source_sampler, uv, layer, variation).r;
}

fn terrain_layer_uv_no_repeat(
    tex_coord: vec2<f32>,
    terrain: TerrainDescription,
    tile_pos: vec2<f32>,
    layer: u32
) -> vec2<f32> {
    // World-continuous UV: continuous across terrain tile borders.
    // Each tile's UV range is offset by tile_pos so the stochastic noise varies between tiles.
    return (tile_pos + tex_coord) * terrain.tiling_scales[layer];
}
