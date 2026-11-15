// ================================================================
// Shared PBR utility functions and types.
// Include with:  #include "core/render/pbr/pbr_functions.wgsl"
// ================================================================
#include "core/render/pbr/pbr_params.wgsl"

// ── SHARED TYPES & STRUCTURES ───────────────────────────────────

struct Camera {
    world_to_view: mat4x4<f32>,
    view_to_ndc: mat4x4<f32>,
    ndc_to_view: mat4x4<f32>,
    view_to_world: mat4x4<f32>,
    position: vec4<f32>
}

struct ShadowParams {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,  // xyz = light direction, w = size of a shadow map texel in world units
    kernel_half_size: f32, // PCF kernel half size in texels
    kernel_scale: f32,     // spacing of the PCF taps in texels
    shadow_intensity: f32,
    depth_range: f32       // depth of the light frustum in world units
}


// ── G-BUFFER OUTPUT ─────────────────────────────────────────────

/// G-buffer fragment output layout shared by all deferred geometry passes.
struct FragOutput {
    @location(0) depth:            f32,        // Linear view-space depth
    @location(1) albedo_metallic:  vec4<f32>,  // (r,g,b) = albedo  a = metallic
    @location(2) normal_roughness: vec4<f32>,  // (r,g,b) = normal  a = roughness
    @location(3) ao:               f32,        // Ambient occlusion (1.0 = fully lit, 0.0 = fully occluded)
    @location(4) emissive:         vec4<f32>,  // (r,g,b) = emitted radiance (HDR)  a = unused
};


// ── MATH UTILITIES ──────────────────────────────────────────────

/// Inverse of a 3×3 matrix via the cross-product adjugate method.
fn mat3_inverse(m: mat3x3<f32>) -> mat3x3<f32> {
    let a = m[0]; let b = m[1]; let c = m[2];
    let r0 = cross(b, c);
    let r1 = cross(c, a);
    let r2 = cross(a, b);
    let inv_det = 1.0 / dot(r0, a);
    return mat3x3<f32>(
        vec3<f32>(r0.x, r1.x, r2.x) * inv_det,
        vec3<f32>(r0.y, r1.y, r2.y) * inv_det,
        vec3<f32>(r0.z, r1.z, r2.z) * inv_det
    );
}

/// Normal matrix = transpose(inverse(upper-left 3×3 of model matrix)).
fn normal_matrix(obj_to_world: mat4x4<f32>) -> mat3x3<f32> {
    return transpose(mat3_inverse(mat3x3<f32>(
        obj_to_world[0].xyz,
        obj_to_world[1].xyz,
        obj_to_world[2].xyz
    )));
}


// ── NORMAL MAPPING ──────────────────────────────────────────────

/// Decode a tangent-space normal map sample and rotate it to world space.
/// `normal_sample` is the raw [0, 1] texture value; returns a unit world-space normal.
fn apply_normal_map(
    normal_sample:   vec3<f32>,
    tangent_world:   vec3<f32>,
    bitangent_world: vec3<f32>,
    normal_world:    vec3<f32>
) -> vec3<f32> {
    let n = normalize(normal_sample * 2.0 - vec3<f32>(1.0));
    let tbn = mat3x3<f32>(
        normalize(tangent_world),
        normalize(bitangent_world),
        normalize(normal_world)
    );
    return normalize(tbn * n);
}


// ── BRDF ────────────────────────────────────────────────────────

const PI: f32 = 3.14159265359;

/// Trowbridge-Reitz GGX normal distribution function (D term).
fn distribution_ggx(n: vec3<f32>, h: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let n_dot_h = max(dot(n, h), 0.0);
    let n_dot_h2 = n_dot_h * n_dot_h;
    var denom = n_dot_h2 * (a2 - 1.0) + 1.0;
    denom = PI * denom * denom;
    return a2 / denom;
}

/// Schlick-GGX geometry sub-term.
fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return n_dot_v / (n_dot_v * (1.0 - k) + k);
}

/// Smith's geometry term combining obstruction and shadowing (G term).
fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    return geometry_schlick_ggx(n_dot_l, roughness)
         * geometry_schlick_ggx(n_dot_v, roughness);
}

/// Fresnel-Schlick approximation (F term).
fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

struct SpecularFresnel {
    specular: vec3<f32>,
    fresnel:  vec3<f32>
};

/// Cook-Torrance specular BRDF. Returns specular color and Fresnel term.
fn specular_cook_torrance(
    n: vec3<f32>, v: vec3<f32>, l: vec3<f32>,
    roughness: f32, f0: vec3<f32>
) -> SpecularFresnel {
    let n_dot_v = max(dot(n, v), 0.0);
    let n_dot_l = max(dot(n, l), 0.0);
    let h       = normalize(v + l);
    let h_dot_v = max(dot(h, v), 0.0);
    let d = distribution_ggx(n, h, roughness);
    let f = fresnel_schlick(h_dot_v, f0);
    let g = geometry_smith(n_dot_v, n_dot_l, roughness);
    let specular = (d * f * g) / (4.0 * n_dot_v * n_dot_l + 0.0001);
    return SpecularFresnel(specular, f);
}

/// Full Cook-Torrance BRDF for one light; returns the outgoing radiance contribution.
fn brdf_for_light(
    n: vec3<f32>, v: vec3<f32>, l: vec3<f32>,
    albedo: vec3<f32>, metallic: f32, roughness: f32, f0: vec3<f32>
) -> vec3<f32> {
    let sf = specular_cook_torrance(n, v, l, roughness, f0);
    let kd = (vec3<f32>(1.0) - sf.fresnel) * (1.0 - metallic);
    return kd * albedo / PI + sf.specular;
}


// ── LIGHT TYPES & EVALUATION ────────────────────────────────────

struct Light {
    /// (x,y,z) = world-space position  w = intensity (lumens)
    position_intensity: vec4<f32>,
    /// (x,y,z) = color  w = type (0 = directional, 1 = point, 2 = spot)
    color_type: vec4<f32>,
    /// (x,y,z) = direction  w = range (max influence distance)
    direction_range: vec4<f32>,
    /// Spot cone: x = inner cos, y = outer cos
    spot_cone: vec4<f32>
};

struct LightData {
    light_dir: vec3<f32>,
    radiance:  vec3<f32>
};

/// Compute light direction and radiance at a given world-space fragment position.
fn get_light_data(light: Light, frag_world_pos: vec3<f32>) -> LightData {
    var light_dir: vec3<f32>;
    var radiance:  vec3<f32>;
    let light_type = i32(light.color_type.w);
    let intensity  = light.position_intensity.w;
    let color      = light.color_type.rgb;

    if light_type == 0 { // Directional
        light_dir = normalize(-light.direction_range.xyz);
        radiance  = color * intensity;
    } else if light_type == 1 { // Point
        let to_frag = light.position_intensity.xyz - frag_world_pos;
        let dist    = length(to_frag);
        light_dir   = normalize(to_frag);
        let atten   = 1.0 / max(dist * dist, 0.01);
        let falloff = smoothstep(light.direction_range.w, 0.0, dist);
        radiance    = color * intensity * atten * falloff;
    } else if light_type == 2 { // Spot
        let to_frag = light.position_intensity.xyz - frag_world_pos;
        let dist    = length(to_frag);
        light_dir   = normalize(to_frag);
        let atten   = 1.0 / max(dist * dist, 0.01);
        let falloff = smoothstep(light.direction_range.w, 0.0, dist);
        let theta   = dot(light_dir, normalize(-light.direction_range.xyz));
        let cone    = smoothstep(light.spot_cone.y, light.spot_cone.x, theta);
        radiance    = color * intensity * atten * falloff * cone;
    }
    return LightData(light_dir, radiance);
}


// ── DEPTH FUNCTIONS ────────────────────────────────────
/// Write linear depth
fn write_depth(z_value: f32) -> f32 {
    return z_value;
}

/// Reconstruct view-space position from linear depth and screen UV coordinates.
/// linear_depth is -view_z (positive distance from camera along the view axis).
fn get_depth_view_position(uv: vec2<f32>, linear_depth: f32, camera: Camera) -> vec3<f32> {
    // Unproject screen UV at ndc_z=0 to get a view-space ray through this pixel.
    let ndc_xy = vec4<f32>(uv.x * 2.0 - 1.0, (1.0 - uv.y) * 2.0 - 1.0, 0.0, 1.0);
    let view_ray = camera.ndc_to_view * ndc_xy;
    let ray = view_ray.xyz / view_ray.w;
    // Scale the ray so its Z component equals -linear_depth.
    return ray * (-linear_depth / ray.z);
}

/// Reconstruct linear view-space depth (positive camera distance) from depth and screen UV.
fn get_depth_camera_distance(uv: vec2<f32>, linear_depth: f32, camera: Camera) -> f32 {
    return linear_depth;
}

/// Reconstruct world-space position from linear depth and screen UV coordinates.
fn get_depth_world_position(uv: vec2<f32>, linear_depth: f32, camera: Camera) -> vec3<f32> {
    let view_pos = get_depth_view_position(uv, linear_depth, camera);
    let world = camera.view_to_world * vec4<f32>(view_pos, 1.0);
    return world.xyz;
}

// Debug views of the meshes (see LodSettings::debug_flags): the level + 1 in the low bits (5 for a mesh
// without levels, 0 when off), then the flags
const MESH_DEBUG_LEVEL_MASK: u32 = 15u;
const MESH_DEBUG_DENSITY: u32 = 16u;
const MESH_DEBUG_WIREFRAME: u32 = 32u;

// Colors of the levels, and of the triangle densities from 1 to 256 pixels per triangle
const MESH_DEBUG_COLORS = array<vec3<f32>, 5>(
    vec3<f32>(0.1, 0.8, 0.1),
    vec3<f32>(0.9, 0.85, 0.1),
    vec3<f32>(1.0, 0.45, 0.05),
    vec3<f32>(0.9, 0.05, 0.05),
    vec3<f32>(0.2, 0.4, 0.95)
);

/// Red, orange, yellow, green then blue from `t` = 0 to 4, clamped.
fn debug_ramp(t: f32) -> vec3<f32> {
    var colors = MESH_DEBUG_COLORS;
    var ramp = array<u32, 5>(3u, 2u, 1u, 0u, 4u);
    let x = clamp(t, 0.0, 3.999);
    let i = u32(x);
    return mix(colors[ramp[i]], colors[ramp[i + 1u]], fract(x));
}

/// Barycentric coordinates of the corner `vertex_index` of its triangle, in a draw without index buffer.
/// To be interpolated linearly in screen space, for `triangle_coverage`.
fn triangle_corner(vertex_index: u32) -> vec3<f32> {
    return select(vec3<f32>(0.0), vec3<f32>(1.0), vec3<u32>(0u, 1u, 2u) == vec3<u32>(vertex_index % 3u));
}

struct TriangleCoverage {
    edge_distance: f32, // To the nearest edge of the triangle, in pixels
    area: f32           // Of the triangle on screen, in pixels
}

/// Coverage of the triangle of a fragment, from its screen-space barycentric coordinates. Takes derivatives:
/// call in uniform control flow.
fn triangle_coverage(barycentric: vec3<f32>) -> TriangleCoverage {
    let dx = dpdx(barycentric);
    let dy = dpdy(barycentric);
    let gradient = max(sqrt(dx * dx + dy * dy), vec3<f32>(1e-8));
    let edge = barycentric / gradient;
    var out: TriangleCoverage;
    out.edge_distance = min(min(edge.x, edge.y), edge.z);
    // The triangle is half the unit square in barycentric space
    out.area = 0.5 / max(abs(dx.x * dy.y - dx.y * dy.x), 1e-12);
    return out;
}

/// Albedo of a mesh in the debug views `debug`, keeping the shading of `albedo` to tell the parts apart in
/// the levels view.
fn mesh_debug_albedo(albedo: vec3<f32>, debug: u32, coverage: TriangleCoverage) -> vec3<f32> {
    var colors = MESH_DEBUG_COLORS;
    var color = albedo;
    let level = debug & MESH_DEBUG_LEVEL_MASK;
    if level != 0u {
        let luma = dot(albedo, vec3<f32>(0.2126, 0.7152, 0.0722));
        color = colors[min(level - 1u, 4u)] * (0.4 + 0.6 * luma);
    }
    if (debug & MESH_DEBUG_DENSITY) != 0u {
        // Every 4 times more pixels per triangle
        color = debug_ramp(log2(coverage.area) * 0.5);
    }
    return color * (1.0 - mesh_debug_wireframe(debug, coverage));
}

/// Opacity of the wireframe over a fragment, 0 when off.
fn mesh_debug_wireframe(debug: u32, coverage: TriangleCoverage) -> f32 {
    if (debug & MESH_DEBUG_WIREFRAME) == 0u {
        return 0.0;
    }
    return 1.0 - smoothstep(0.25, 1.0, coverage.edge_distance);
}
