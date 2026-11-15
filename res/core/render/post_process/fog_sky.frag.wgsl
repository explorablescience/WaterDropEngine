#include "core/render/atmosphere/sky.wgsl"
#include "core/render/post_process/fog_common.wgsl"

@group(2) @binding(1) var<uniform> in_atmosphere: AtmosphereParams;

// Fog color, mixed with the color of the sky horizon in the ray direction
fn fog_base_color(ray: vec3<f32>) -> vec3<f32> {
    let flat_ray = vec3<f32>(ray.x, 0.0, ray.z);
    let flat_length = length(flat_ray);
    let horizon_ray = select(vec3<f32>(1.0, 0.0, 0.0), flat_ray / flat_length, flat_length > 1e-4);
    return mix(in_fog.color.rgb, sky_gradient(horizon_ray, in_atmosphere), in_fog.color.w);
}
