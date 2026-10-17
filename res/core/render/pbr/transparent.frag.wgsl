#include "core/render/pbr/pbr_functions.wgsl"
#include "core/render/atmosphere/mars_sky.wgsl"

// Forward lighting for alpha-blended PBR materials. Mirrors lighting.frag.wgsl (without shadows)
// so glass matches the deferred opaque surfaces.

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord:    vec2<f32>,
    @location(1) normal_world: vec3<f32>,
    @location(2) world_pos:    vec3<f32>,
};

@group(1) @binding(0) var<uniform> in_camera: Camera;

struct Material3dUniform {
    flags: vec4<f32>,
    albedo: vec4<f32>,
    metallic: f32,
    roughness: f32,
    _padding: vec2<f32>,
};
@group(3) @binding(0) var<uniform> in_pbr_material: Material3dUniform;
@group(3) @binding(1) var in_albedo_texture: texture_2d<f32>;
@group(3) @binding(2) var in_albedo_sampler: sampler;

@group(1) @binding(1) var<storage, read> in_lights: array<Light>;
@group(1) @binding(2) var<uniform> in_atmosphere: AtmosphereParams;
@group(1) @binding(3) var<uniform> in_pbr_params: PbrParams;

@fragment
fn main(in: VertexOutput, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    var albedo = in_pbr_material.albedo;
    if (in_pbr_material.flags.x == 1.0) {
        albedo = textureSample(in_albedo_texture, in_albedo_sampler, in.tex_coord);
    }
    let metallic  = clamp(in_pbr_material.metallic * in_pbr_params.material_params.x, 0.0, 1.0);
    let roughness = clamp(in_pbr_material.roughness * in_pbr_params.material_params.y, 0.0, 1.0);

    var normal = normalize(in.normal_world);
    if (!front_facing) {
        normal = -normal;
    }
    let view_dir = normalize(in_camera.position.xyz - in.world_pos);
    let f0 = mix(vec3<f32>(0.04), albedo.rgb, metallic);

    var lo = vec3<f32>(0.0);
    let lights_count = i32(arrayLength(&in_lights));
    for (var i = 0; i < lights_count; i = i + 1) {
        let light_data = get_light_data(in_lights[i], in.world_pos);
        let n_dot_l = max(dot(normal, light_data.light_dir), 0.0);
        lo += brdf_for_light(normal, view_dir, light_data.light_dir, albedo.rgb, metallic, roughness, f0)
            * light_data.radiance
            * n_dot_l
            * in_pbr_params.lighting_params.x;
    }

    let irradiance = mars_sky(normal, in_atmosphere) * in_pbr_params.lighting_params.y;
    let ambient = 0.2 * irradiance * albedo.rgb;

    // Glass reflects more at grazing angles: raise opacity and add a sky reflection there
    let n_dot_v = clamp(dot(normal, view_dir), 0.0, 1.0);
    let fresnel = pow(1.0 - n_dot_v, 5.0);
    let reflection = mars_sky(reflect(-view_dir, normal), in_atmosphere) * in_pbr_params.lighting_params.y
        * fresnel * (1.0 - roughness) * 0.2;
    let alpha = clamp(albedo.a + fresnel * (1.0 - albedo.a) * 0.6, 0.0, 1.0);

    var color = lo + ambient + reflection;
    color = color * in_pbr_params.lighting_params.z;
    color = color / (color + vec3<f32>(in_pbr_params.lighting_params.w));
    return vec4<f32>(color, alpha);
}
