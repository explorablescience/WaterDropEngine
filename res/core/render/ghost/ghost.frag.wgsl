struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord:    vec2<f32>,
    @location(1) normal_world: vec3<f32>,
    @location(2) view_dir:     vec3<f32>,
    @location(3) world_pos:    vec3<f32>,
};

struct GhostMaterialUniform {
    overlay: vec4<f32>, // rgb = tint (valid / invalid color), a = base opacity
    albedo: vec4<f32>,
    params: vec4<f32>,  // (albedo_texture_present, desaturate_factor, rim_power, rim_strength)
};
@group(3) @binding(0) var<uniform> in_ghost_material: GhostMaterialUniform;
@group(3) @binding(1) var in_albedo_texture: texture_2d<f32>;
@group(3) @binding(2) var in_albedo_sampler: sampler;

// Perceptual luminance (BT.709)
fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// Holographic placement preview: the model's colors, desaturated and tinted, simply shaded so the
// volumes read, with a bright fresnel rim and faint scanlines.
@fragment
fn main(in: VertexOutput, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let desaturate = in_ghost_material.params.y;
    let rim_power = in_ghost_material.params.z;
    let rim_strength = in_ghost_material.params.w;
    let tint = in_ghost_material.overlay.rgb;
    let base_alpha = in_ghost_material.overlay.a;

    var albedo = in_ghost_material.albedo.rgb;
    if (in_ghost_material.params.x > 0.5) {
        albedo = textureSample(in_albedo_texture, in_albedo_sampler, in.tex_coord).rgb;
    }

    var normal = normalize(in.normal_world);
    if (!front_facing) {
        normal = -normal;
    }
    let view_dir = normalize(in.view_dir);

    // Soft key light from above plus ambient, so shapes keep their depth
    let light_dir = normalize(vec3<f32>(0.35, 0.85, 0.4));
    let shading = 0.55 + 0.45 * max(dot(normal, light_dir), 0.0);

    // Desaturated model color pulled toward the tint
    let gray = vec3<f32>(luminance(albedo));
    let body = mix(mix(albedo, gray, desaturate), tint, 0.45) * shading;

    // Fresnel rim
    let n_dot_v = clamp(abs(dot(normal, view_dir)), 0.0, 1.0);
    let rim = pow(1.0 - n_dot_v, rim_power) * rim_strength;

    // Faint horizontal scanlines in world space
    let scan = 0.9 + 0.1 * step(0.5, fract(in.world_pos.y * 12.0));

    let color = (body + tint * rim) * scan;
    let alpha = clamp(base_alpha + rim * (1.0 - base_alpha), 0.0, 0.95);
    return vec4<f32>(color, alpha);
}
