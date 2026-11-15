#include "core/render/debug/view_common.wgsl"

@group(0) @binding(0) var in_source: texture_2d<f32>;

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let coord = texel(in.uv, textureDimensions(in_source));
    return display(textureLoad(in_source, coord, 0));
}
