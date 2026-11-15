// Projects the sky on L2 spherical harmonics, convolved with the cosine lobe for the diffuse lighting.
#include "core/render/atmosphere/sky.wgsl"
#include "core/render/pbr/environment.wgsl"

@group(0) @binding(0) var<uniform> in_atmosphere: AtmosphereParams;
@group(0) @binding(1) var<storage, read_write> out_sh: array<vec4<f32>, 9>;

const THREADS: u32 = 64u;
const SAMPLES: u32 = 8192u;

var<workgroup> partial: array<array<vec4<f32>, 9>, THREADS>;

// Evenly spread directions over the sphere.
fn fibonacci_direction(i: u32) -> vec3<f32> {
    let y = 1.0 - (f32(i) + 0.5) * 2.0 / f32(SAMPLES);
    let radius = sqrt(max(1.0 - y * y, 0.0));
    let phi = f32(i) * 2.39996323;
    return vec3<f32>(cos(phi) * radius, y, sin(phi) * radius);
}

@compute @workgroup_size(THREADS, 1, 1)
fn main(@builtin(local_invocation_index) index: u32) {
    var sum: array<vec4<f32>, 9>;
    for (var i = index; i < SAMPLES; i += THREADS) {
        let direction = fibonacci_direction(i);
        let radiance = vec4<f32>(sky_environment(direction, in_atmosphere), 0.0);
        let basis = sh_basis(direction);
        for (var k = 0u; k < 9u; k++) {
            sum[k] += radiance * basis[k];
        }
    }
    partial[index] = sum;
    workgroupBarrier();

    for (var stride = THREADS / 2u; stride > 0u; stride /= 2u) {
        if index < stride {
            for (var k = 0u; k < 9u; k++) {
                partial[index][k] += partial[index + stride][k];
            }
        }
        workgroupBarrier();
    }

    if index < 9u {
        // Cosine lobe convolution divided by π, per band: 1, 2/3, 1/4
        var band = 0.25;
        if index == 0u {
            band = 1.0;
        } else if index < 4u {
            band = 2.0 / 3.0;
        }
        let solid_angle = 4.0 * PI / f32(SAMPLES);
        out_sh[index] = partial[0][index] * solid_angle * band;
    }
}
