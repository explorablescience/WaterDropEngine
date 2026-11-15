#include "core/render/debug/view_common.wgsl"

@group(0) @binding(0) var in_source: texture_depth_2d;

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let coord = texel(in.uv, textureDimensions(in_source));
    return display(vec4<f32>(vec3<f32>(textureLoad(in_source, coord, 0)), 1.0));
}
