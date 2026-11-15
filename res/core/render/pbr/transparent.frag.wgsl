#include "core/render/pbr/pbr_functions.wgsl"
#include "core/render/pbr/environment.wgsl"

// Forward lighting for alpha-blended PBR materials. Mirrors lighting.frag.wgsl (without shadows)
// so glass matches the deferred opaque surfaces.

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord:    vec2<f32>,
    @location(1) normal_world: vec3<f32>,
    @location(2) world_pos:    vec3<f32>,
    @location(3) @interpolate(flat) debug: u32,
    @location(4) @interpolate(linear) barycentric: vec3<f32>,
};

@group(1) @binding(0) var<uniform> in_camera: Camera;

struct Material3dUniform {
    flags: vec4<f32>,
    albedo: vec4<f32>,
    emissive: vec4<f32>,
    metallic: f32,
    roughness: f32,
    _padding: vec2<f32>,
};
@group(3) @binding(0) var<uniform> in_pbr_material: Material3dUniform;
@group(3) @binding(1) var in_albedo_texture: texture_2d<f32>;
@group(3) @binding(2) var in_albedo_sampler: sampler;
@group(3) @binding(9) var in_emissive_texture: texture_2d<f32>;
@group(3) @binding(10) var in_emissive_sampler: sampler;

@group(1) @binding(1) var<storage, read> in_lights: array<Light>;
@group(1) @binding(3) var<uniform> in_pbr_params: PbrParams;
@group(1) @binding(4) var<uniform> in_environment_sh: EnvironmentSH;
@group(1) @binding(5) var in_environment_map: texture_cube<f32>;
@group(1) @binding(6) var in_environment_sampler: sampler;
@group(1) @binding(7) var in_environment_brdf: texture_2d<f32>;

@fragment
fn main(in: VertexOutput, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let coverage = triangle_coverage(in.barycentric);
    var albedo = in_pbr_material.albedo;
    if (in_pbr_material.flags.x == 1.0) {
        albedo = textureSample(in_albedo_texture, in_albedo_sampler, in.tex_coord);
    }
    if (in.debug != 0u) {
        albedo = vec4<f32>(mesh_debug_albedo(albedo.rgb, in.debug, coverage), albedo.a);
    }
    var metallic  = clamp(in_pbr_material.metallic * in_pbr_params.material_params.x, 0.0, 1.0);

    // The data debug views show the opaque surfaces behind
    let debug_view = pbr_debug_view(in_pbr_params);
    if pbr_debug_is_data(debug_view) {
        discard;
    }
    if debug_view == PBR_DEBUG_CLAY {
        albedo = vec4<f32>(vec3<f32>(PBR_DEBUG_CLAY_ALBEDO), albedo.a);
        metallic = 0.0;
    }
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
        let light_type = i32(in_lights[i].color_type.w);
        if light_type < 0 {
            break; // End of the lights in use
        }
        if light_type > 0 && distance(in_lights[i].position_intensity.xyz, in.world_pos) > in_lights[i].direction_range.w {
            continue; // Out of range
        }
        let light_data = get_light_data(in_lights[i], in.world_pos);
        let n_dot_l = max(dot(normal, light_data.light_dir), 0.0);
        lo += brdf_for_light(normal, view_dir, light_data.light_dir, albedo.rgb, metallic, roughness, f0)
            * light_data.radiance
            * n_dot_l
            * in_pbr_params.lighting_params.x;
    }

    // Ambient light of the environment, with its reflection
    let ambient = environment_lighting(
        in_environment_sh, in_environment_map, in_environment_sampler, in_environment_brdf,
        normal, view_dir, albedo.rgb, metallic, roughness, f0, 1.0
    );
    let ambient_scale = in_pbr_params.lighting_params.y;

    // Glass reflects more at grazing angles: raise opacity there
    let n_dot_v = clamp(dot(normal, view_dir), 0.0, 1.0);
    let fresnel = pow(1.0 - n_dot_v, 5.0);
    let alpha = clamp(albedo.a + fresnel * (1.0 - albedo.a) * 0.6, 0.0, 1.0);

    var emissive = in_pbr_material.emissive.rgb;
    if (in_pbr_material.emissive.w == 1.0) {
        emissive *= textureSample(in_emissive_texture, in_emissive_sampler, in.tex_coord).rgb;
    }

    var color = pbr_debug_light(debug_view, lo, ambient.diffuse * ambient_scale, ambient.specular * ambient_scale, emissive * (1.0 - mesh_debug_wireframe(in.debug, coverage)));
    color = color * in_pbr_params.lighting_params.z;
    return vec4<f32>(color, alpha);
}
