struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord:    vec2<f32>,
    @location(1) normal_world: vec3<f32>,
    @location(2) view_dir:     vec3<f32>,
};

struct GhostMaterialUniform {
    overlay: vec4<f32>,
    albedo: vec4<f32>,
    params: vec4<f32>, // (albedo_texture_present, desaturate_factor, rim_power, rim_strength)
};
@group(3) @binding(0) var<uniform> in_ghost_material: GhostMaterialUniform;
@group(3) @binding(1) var in_albedo_texture: texture_2d<f32>;
@group(3) @binding(2) var in_albedo_sampler: sampler;


// Perceptual luminance (BT.709)
fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let is_albedo_texture = in_ghost_material.params.x;
    let desaturate = in_ghost_material.params.y;
    let rim_power = in_ghost_material.params.z;
    let rim_strength = in_ghost_material.params.w;

    // Sample albedo color
    var albedo = in_ghost_material.albedo;
    if (is_albedo_texture > 0.5) {
        albedo = textureSample(in_albedo_texture, in_albedo_sampler, in.tex_coord);
    }

    // Desaturate toward gray based on the desaturate factor
    let gray_color  = vec3<f32>(luminance(albedo.rgb));
    let desat_color = mix(albedo.rgb, gray_color, desaturate);

    // Blend with overlay color (using alpha for blending)
    let blended_color = mix(desat_color, in_ghost_material.overlay.rgb, in_ghost_material.overlay.a);

    // Add Fresnel rim effect based on view direction and normal
    let n_dot_v = saturate(dot(in.normal_world, -in.view_dir));
    let rim_factor = pow(1.0 - n_dot_v, rim_power) * rim_strength;

    // Ghost alpha (base transparency from overlay alpha + boosted at edges)
    let base_alpha = in_ghost_material.overlay.a;
    let ghost_alpha = saturate(base_alpha + rim_factor * rim_strength);

    // Bake final color with computed alpha
    let final_rgb = mix(blended_color, in_ghost_material.overlay.rgb, rim_factor * rim_strength);
    return vec4<f32>(final_rgb, ghost_alpha);
}
