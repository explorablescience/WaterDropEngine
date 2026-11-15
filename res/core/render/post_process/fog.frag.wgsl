#include "core/render/post_process/fog_common.wgsl"

fn fog_base_color(ray: vec3<f32>) -> vec3<f32> {
    return in_fog.color.rgb;
}
