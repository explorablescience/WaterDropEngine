// ================================================================
// Shared PBR utility functions and types.
// Include with:  #include "core/render/pbr/pbr_functions.wgsl"
// ================================================================

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
    light_dir: vec4<f32>,  // xyz = direction toward sun, w = depth bias
    kernel_half_size: f32, // kernel half size for PCF sampling for each cascade (x, y, z) and unused (w)
    kernel_scale: f32,     // kernel scale for PCF sampling for each cascade (x, y, z) and unused (w)
    shadow_intensity: f32,
    _padding: f32
}


// ── G-BUFFER OUTPUT ─────────────────────────────────────────────

/// G-buffer fragment output layout shared by all deferred geometry passes.
struct FragOutput {
    @location(0) depth:            f32,        // Linear view-space depth
    @location(1) albedo_metallic:  vec4<f32>,  // (r,g,b) = albedo  a = metallic
    @location(2) normal_roughness: vec4<f32>,  // (r,g,b) = normal  a = roughness
    @location(3) ao:               f32,        // Ambient occlusion (1.0 = fully lit, 0.0 = fully occluded)
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

/// Runtime parameters controlling deferred PBR lighting.
struct PbrParams {
    /// x=direct_scale, y=ambient_scale, z=exposure, w=tone_map_white
    lighting_params: vec4<f32>,
    /// x=metallic_scale, y=roughness_scale, z=min_roughness
    material_params: vec4<f32>
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
