#include "core/render/atmosphere/sky.wgsl"
#include "core/render/pbr/pbr_params.wgsl"

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>
};

struct Camera {
    world_to_view: mat4x4<f32>,
    view_to_ndc:   mat4x4<f32>,
    ndc_to_view:   mat4x4<f32>,
    view_to_world: mat4x4<f32>,
    position:      vec4<f32>
}
@group(0) @binding(0) var<uniform> in_camera: Camera;

// Linear view-space depth written by the GBuffer pass, 0 where there is no geometry
@group(1) @binding(0) var in_depth_t: texture_2d<f32>;
@group(2) @binding(1) var<uniform> in_atmosphere: AtmosphereParams;
@group(2) @binding(2) var<uniform> in_pbr_params: PbrParams;

// Reconstruct the world-space view ray for this fragment.
fn view_ray(uv: vec2<f32>) -> vec3<f32> {
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, (1.0 - uv.y) * 2.0 - 1.0);
    let clip = in_camera.ndc_to_view * vec4<f32>(ndc, 1.0, 1.0);
    let view_dir = normalize(clip.xyz / clip.w);
    return normalize((in_camera.view_to_world * vec4<f32>(view_dir, 0.0)).xyz);
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Skip pixels that already have geometry written by the PBR passes.
    let depth = textureLoad(in_depth_t, vec2<i32>(in.clip_position.xy), 0).r;
    if depth > 0.0 {
        discard;
    }
    // The debug views of the lighting only show surfaces
    if pbr_debug_view(in_pbr_params) != PBR_DEBUG_OFF {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let ray = view_ray(in.tex_coord);
    let color = sky(ray, in_atmosphere);
    return vec4<f32>(color, 1.0);
}
