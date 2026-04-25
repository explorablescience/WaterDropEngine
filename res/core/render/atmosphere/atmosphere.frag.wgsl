#include "core/render/atmosphere/mars_sky.wgsl"

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

// Resolved (non-MSAA) linear view-space depth written by the GBuffer pass.
// Pixels with no geometry have depth == 0.
@group(1) @binding(0) var in_depth_t: texture_2d<f32>;
@group(1) @binding(1) var in_depth_s: sampler;
@group(2) @binding(1) var<uniform> in_atmosphere: AtmosphereParams;

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
    let depth = textureSample(in_depth_t, in_depth_s, in.tex_coord).r;
    if depth > 0.0 {
        discard;
    }

    let ray = view_ray(in.tex_coord);
    let color = mars_sky(ray, in_atmosphere);
    return vec4<f32>(color, 1.0);
}
