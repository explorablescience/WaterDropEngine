#include "core/render/pbr/pbr_functions.wgsl"
#include "core/render/terrain/terrain_common.wgsl"

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) ndc_z:     f32,
    @location(3) @interpolate(flat) tile_layer: u32
};

@group(1) @binding(0) var material_albedo:               texture_2d_array<f32>;
@group(1) @binding(1) var material_albedo_sampler:       sampler;
@group(1) @binding(2) var material_normal:               texture_2d_array<f32>;
@group(1) @binding(3) var material_normal_sampler:       sampler;
@group(1) @binding(4) var material_roughness:            texture_2d_array<f32>;
@group(1) @binding(5) var material_roughness_sampler:    sampler;
@group(1) @binding(6) var material_ao:                   texture_2d_array<f32>;
@group(1) @binding(7) var material_ao_sampler:           sampler;
@group(1) @binding(8) var material_displacement:         texture_2d_array<f32>;
@group(1) @binding(9) var material_displacement_sampler: sampler;

@group(2) @binding(0) var<uniform> in_terrain: TerrainDescription;

@group(3) @binding(0) var in_heightmap:          texture_2d_array<f32>;
@group(3) @binding(1) var in_splatmap_a:         texture_2d_array<f32>;
@group(3) @binding(2) var in_splatmap_a_sampler: sampler;
@group(3) @binding(3) var in_splatmap_b:         texture_2d_array<f32>;
@group(3) @binding(4) var in_splatmap_b_sampler: sampler;
// Slope angle over 90°, and whether buildable
@group(3) @binding(5) var in_masks:              texture_2d_array<f32>;
@group(3) @binding(6) var in_masks_sampler:      sampler;

// Slope (1 - normal.y) from which materials are also projected on the vertical planes
const TRIPLANAR_START: f32 = 0.2;
const TRIPLANAR_FULL:  f32 = 0.35;
// Weights below this are not sampled
const MIN_WEIGHT: f32 = 0.01;
// Relief range over which neighbor materials mix in height blending
const BLEND_DEPTH: f32 = 0.2;
// Strength of the material normal maps
const NORMAL_STRENGTH: f32 = 0.7;
// Second sampling of the materials from above, larger and rotated, hiding their repetition
const LARGE_SCALE: f32 = 0.31;
const LARGE_ROTATION: mat2x2<f32> = mat2x2<f32>(0.8, 0.6, -0.6, 0.8);
const LARGE_MIX: f32 = 0.45;
// Opacity of the debug views over the ground
const DEBUG_MIX: f32 = 0.65;

// ── TERRAIN NORMAL ──────────────────────────────────────────────

fn texel_height(p: vec2<i32>, layer: i32) -> f32 {
    return textureLoad(in_heightmap, p, layer, 0).r * in_terrain.tile_size.y;
}

fn height_gradient(p: vec2<i32>, layer: i32) -> vec2<f32> {
    let spacing = in_terrain.tile_size.x / in_terrain.tile_subdivisions;
    return vec2<f32>(
        texel_height(p + vec2<i32>(1, 0), layer) - texel_height(p - vec2<i32>(1, 0), layer),
        texel_height(p + vec2<i32>(0, 1), layer) - texel_height(p - vec2<i32>(0, 1), layer)
    ) / (2.0 * spacing);
}

// Smooth normal at the fragment, from the heightmap gradients (independent of the mesh detail)
fn terrain_normal(tex_coord: vec2<f32>, layer: i32) -> vec3<f32> {
    let n  = in_terrain.tile_subdivisions;
    let f  = tex_coord * n + 1.0;
    let p0 = clamp(vec2<i32>(floor(f)), vec2<i32>(1), vec2<i32>(i32(n)));
    let t  = f - vec2<f32>(p0);
    let g = mix(
        mix(height_gradient(p0, layer), height_gradient(p0 + vec2<i32>(1, 0), layer), t.x),
        mix(height_gradient(p0 + vec2<i32>(0, 1), layer), height_gradient(p0 + vec2<i32>(1, 1), layer), t.x),
        t.y
    );
    return normalize(vec3<f32>(-g.x, 1.0, -g.y));
}

// ── MACRO VARIATION ─────────────────────────────────────────────

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y
    );
}

// Low-frequency brightness variation, hiding the repetition of the material textures
fn macro_variation(xz: vec2<f32>) -> f32 {
    return value_noise(xz * 0.011) * 0.55 + value_noise(xz * 0.043) * 0.3 + value_noise(xz * 0.17) * 0.15;
}

// ── DEBUG VIEWS ─────────────────────────────────────────────────

fn debug_tint(masks: vec2<f32>) -> vec3<f32> {
    let angle = masks.r;
    let walk  = in_terrain.walk_max_angle;
    let steep = step(walk, angle);
    if in_terrain.debug_view < 1.5 {
        // Buildable: green, outlined; walkable: sand; too steep to walk: red
        let buildable = smoothstep(0.45, 0.55, masks.g);
        let outline = 1.0 - smoothstep(0.0, fwidth(masks.g) * 1.5, abs(masks.g - 0.5));
        var color = mix(vec3<f32>(0.9, 0.75, 0.45), vec3<f32>(0.85, 0.12, 0.08), steep);
        color = mix(color, vec3<f32>(0.15, 0.8, 0.25), buildable);
        return mix(color, vec3<f32>(0.02, 0.2, 0.05), outline * 0.8);
    }
    // Slope: blue (flat), green, yellow (steepest walkable), then red; a contour every 5°
    let t = clamp(angle / walk, 0.0, 1.0);
    var color = mix(vec3<f32>(0.1, 0.3, 0.9), vec3<f32>(0.1, 0.8, 0.3), smoothstep(0.0, 0.5, t));
    color = mix(color, vec3<f32>(0.95, 0.85, 0.1), smoothstep(0.5, 1.0, t));
    color = mix(color, vec3<f32>(0.85, 0.1, 0.08), steep);
    let contour = abs(fract(angle * 18.0 + 0.5) - 0.5) / max(fwidth(angle * 18.0), 1e-4);
    return color * mix(0.6, 1.0, smoothstep(0.5, 1.5, contour));
}

// ── MATERIALS ───────────────────────────────────────────────────

struct Layer {
    albedo:    vec3<f32>,
    normal_ts: vec3<f32>,
    roughness: f32,
    ao:        f32,
    height:    f32
}

struct MaterialSample {
    albedo:    vec3<f32>,
    normal:    vec3<f32>,  // world space
    roughness: f32,
    ao:        f32,
    height:    f32
}

// Explicit gradients: the material is sampled in non-uniform control flow
fn sample_layer(m: u32, uv: vec2<f32>, ddx: vec2<f32>, ddy: vec2<f32>) -> Layer {
    var l: Layer;
    l.albedo    = textureSampleGrad(material_albedo,       material_albedo_sampler,       uv, m, ddx, ddy).rgb;
    l.normal_ts = textureSampleGrad(material_normal,       material_normal_sampler,       uv, m, ddx, ddy).rgb * 2.0 - 1.0;
    l.normal_ts = vec3<f32>(l.normal_ts.xy * NORMAL_STRENGTH, l.normal_ts.z);
    l.roughness = textureSampleGrad(material_roughness,    material_roughness_sampler,    uv, m, ddx, ddy).r;
    l.ao        = textureSampleGrad(material_ao,           material_ao_sampler,           uv, m, ddx, ddy).r;
    l.height    = textureSampleGrad(material_displacement, material_displacement_sampler, uv, m, ddx, ddy).r;
    return l;
}

// Tangent-space normal (OpenGL convention) projected along the given tangent axes
fn perturb(n: vec3<f32>, tangent_axis: vec3<f32>, bitangent_axis: vec3<f32>, ts: vec3<f32>) -> vec3<f32> {
    let t = normalize(tangent_axis - n * dot(n, tangent_axis));
    let b = normalize(cross(n, t) * sign(dot(cross(n, t), bitangent_axis)));
    return normalize(t * ts.x + b * ts.y + n * ts.z);
}

// Material in world space: projected from above, and on slopes also from the sides (triplanar)
fn sample_material(m: u32, world: vec3<f32>, n: vec3<f32>, dx: vec3<f32>, dy: vec3<f32>, triplanar: f32) -> MaterialSample {
    let scale = 1.0 / material_value(in_terrain.tiling, m);
    var top = sample_layer(m, world.xz * scale, dx.xz * scale, dy.xz * scale);

    // Blend in a larger, rotated sampling: the repetition of the textures no longer shows
    let large = LARGE_ROTATION * (scale * LARGE_SCALE);
    let large_uv = large * world.xz + vec2<f32>(0.37, 0.71) * f32(m + 1u);
    let large_albedo = textureSampleGrad(material_albedo, material_albedo_sampler, large_uv, m, large * dx.xz, large * dy.xz).rgb;
    var large_normal = textureSampleGrad(material_normal, material_normal_sampler, large_uv, m, large * dx.xz, large * dy.xz).rgb * 2.0 - 1.0;
    // The large normals are rotated with their texture
    large_normal = vec3<f32>(transpose(LARGE_ROTATION) * large_normal.xy * NORMAL_STRENGTH, large_normal.z);
    top.albedo    = mix(top.albedo, large_albedo, LARGE_MIX);
    top.normal_ts = normalize(vec3<f32>(top.normal_ts.xy + large_normal.xy, top.normal_ts.z * large_normal.z));

    var out: MaterialSample;
    out.albedo    = top.albedo;
    out.normal    = perturb(n, vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), top.normal_ts);
    out.roughness = top.roughness;
    out.ao        = top.ao;
    out.height    = top.height;
    if triplanar <= 0.0 {
        return out;
    }

    let side_x = sample_layer(m, world.zy * scale, dx.zy * scale, dy.zy * scale);
    let side_z = sample_layer(m, world.xy * scale, dx.xy * scale, dy.xy * scale);
    var w = pow(abs(n), vec3<f32>(4.0));
    w = w / (w.x + w.y + w.z);
    let albedo    = side_x.albedo * w.x + top.albedo * w.y + side_z.albedo * w.z;
    let roughness = side_x.roughness * w.x + top.roughness * w.y + side_z.roughness * w.z;
    let ao        = side_x.ao * w.x + top.ao * w.y + side_z.ao * w.z;
    let height    = side_x.height * w.x + top.height * w.y + side_z.height * w.z;
    let normal = normalize(
        perturb(n, vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(0.0, 1.0, 0.0), side_x.normal_ts) * w.x
        + out.normal * w.y
        + perturb(n, vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), side_z.normal_ts) * w.z
    );
    out.albedo    = mix(out.albedo, albedo, triplanar);
    out.normal    = normalize(mix(out.normal, normal, triplanar));
    out.roughness = mix(out.roughness, roughness, triplanar);
    out.ao        = mix(out.ao, ao, triplanar);
    out.height    = mix(out.height, height, triplanar);
    return out;
}

@fragment
fn main(in: VertexOutput) -> FragOutput {
    var out: FragOutput;
    let layer = i32(in.tile_layer);
    let n = terrain_normal(in.tex_coord, layer);
    let dx = dpdx(in.world_pos);
    let dy = dpdy(in.world_pos);

    // Material weights, sampled at the texel centers of the chunk samples
    let splat_uv = (in.tex_coord * in_terrain.tile_subdivisions + 1.5) / in_terrain.map_size;
    let wa = textureSample(in_splatmap_a, in_splatmap_a_sampler, splat_uv, layer);
    let wb = textureSample(in_splatmap_b, in_splatmap_b_sampler, splat_uv, layer);
    var weights = array<f32, 8>(wa.x, wa.y, wa.z, wa.w, wb.x, wb.y, wb.z, wb.w);
    var total = 0.0;
    for (var m = 0u; m < TERRAIN_MATERIALS; m++) {
        total += weights[m];
    }

    // Sample the materials in use; each one's relief raises it over the others
    let triplanar = smoothstep(TRIPLANAR_START, TRIPLANAR_FULL, 1.0 - n.y);
    var samples: array<MaterialSample, 8>;
    var raised: array<f32, 8>;
    var highest = -1e9;
    for (var m = 0u; m < TERRAIN_MATERIALS; m++) {
        weights[m] = weights[m] / max(total, 1e-4);
        if weights[m] > MIN_WEIGHT {
            samples[m] = sample_material(m, in.world_pos, n, dx, dy, triplanar);
            raised[m]  = weights[m] + samples[m].height * material_value(in_terrain.height_blend, m) * 0.5;
            highest    = max(highest, raised[m]);
        }
    }

    // Height blending: near transitions, the material standing out wins
    var sum       = 0.0;
    var albedo    = vec3<f32>(0.0);
    var normal    = vec3<f32>(0.0);
    var roughness = 0.0;
    var ao        = 0.0;
    for (var m = 0u; m < TERRAIN_MATERIALS; m++) {
        if weights[m] > MIN_WEIGHT {
            let w = max(raised[m] - highest + BLEND_DEPTH, 0.0);
            albedo    += samples[m].albedo * w;
            normal    += samples[m].normal * w;
            roughness += samples[m].roughness * w;
            ao        += samples[m].ao * w;
            sum       += w;
        }
    }
    sum = max(sum, 1e-4);

    // Broad brightness and hue variation across the plains
    albedo *= mix(vec3<f32>(0.8, 0.84, 0.9), vec3<f32>(1.12, 1.05, 0.98), macro_variation(in.world_pos.xz));
    albedo = albedo / sum;

    let masks = textureSample(in_masks, in_masks_sampler, splat_uv, layer).rg;
    if in_terrain.debug_view > 0.5 {
        albedo = mix(albedo, debug_tint(masks), DEBUG_MIX);
    }
    out.albedo_metallic  = vec4<f32>(albedo, 0.0);
    // Martian dust and rock are matte
    out.normal_roughness = vec4<f32>(normalize(normal / sum), mix(0.6, 1.0, roughness / sum));
    out.ao    = ao / sum;
    out.depth = write_depth(in.ndc_z);
    return out;
}
