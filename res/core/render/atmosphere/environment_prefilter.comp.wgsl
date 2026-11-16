// Prefilters the sky into a mip of the environment map, for the specular lighting of a roughness (GGX, with n = v = r).
// The sky is evaluated along each sample, so no source map is needed.
#include "core/render/atmosphere/sky.wgsl"
#include "core/render/pbr/pbr_functions.wgsl"

@group(0) @binding(0) var<uniform> in_atmosphere: AtmosphereParams;
@group(0) @binding(2) var out_map: texture_storage_2d_array<rgba16float, write>;

struct PrefilterParams {
    roughness: f32
}
var<immediate> in_params: PrefilterParams;

const SAMPLES: u32 = 256u;

// Direction through a texel of a cube face, uv in [-1, 1] (faces +X, -X, +Y, -Y, +Z, -Z).
fn cube_direction(face: u32, uv: vec2<f32>) -> vec3<f32> {
    switch face {
        case 0u: { return normalize(vec3<f32>(1.0, -uv.y, -uv.x)); }
        case 1u: { return normalize(vec3<f32>(-1.0, -uv.y, uv.x)); }
        case 2u: { return normalize(vec3<f32>(uv.x, 1.0, uv.y)); }
        case 3u: { return normalize(vec3<f32>(uv.x, -1.0, -uv.y)); }
        case 4u: { return normalize(vec3<f32>(uv.x, -uv.y, 1.0)); }
        default: { return normalize(vec3<f32>(-uv.x, -uv.y, -1.0)); }
    }
}

fn hammersley(i: u32) -> vec2<f32> {
    return vec2<f32>(f32(i) / f32(SAMPLES), f32(reverseBits(i)) * 2.3283064365386963e-10);
}

// Half vector around n, distributed following the GGX distribution.
fn importance_sample_ggx(xi: vec2<f32>, n: vec3<f32>, alpha: f32) -> vec3<f32> {
    let phi = 2.0 * PI * xi.x;
    let cos_theta = sqrt((1.0 - xi.y) / (1.0 + (alpha * alpha - 1.0) * xi.y));
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
    let up = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), abs(n.z) < 0.999);
    let tangent = normalize(cross(up, n));
    let bitangent = cross(n, tangent);
    return normalize(tangent * (cos(phi) * sin_theta) + bitangent * (sin(phi) * sin_theta) + n * cos_theta);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(out_map).x;
    if id.x >= size || id.y >= size {
        return;
    }
    let uv = (vec2<f32>(id.xy) + 0.5) / f32(size) * 2.0 - 1.0;
    let n = cube_direction(id.z, uv);

    var color = vec3<f32>(0.0);
    if in_params.roughness <= 0.0 {
        color = sky_environment(n, in_atmosphere);
    } else {
        let alpha = in_params.roughness * in_params.roughness;
        var weight = 0.0;
        for (var i = 0u; i < SAMPLES; i++) {
            let h = importance_sample_ggx(hammersley(i), n, alpha);
            let l = normalize(2.0 * dot(n, h) * h - n);
            let n_dot_l = dot(n, l);
            if n_dot_l > 0.0 {
                color += sky_environment(l, in_atmosphere) * n_dot_l;
                weight += n_dot_l;
            }
        }
        color /= max(weight, 1e-4);
    }
    textureStore(out_map, id.xy, id.z, vec4<f32>(color, 1.0));
}
