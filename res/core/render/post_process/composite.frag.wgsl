#include "core/render/post_process/post_process_params.wgsl"
#include "core/render/post_process/tonemapping.wgsl"

@group(0) @binding(0) var in_hdr_t: texture_2d<f32>;
@group(0) @binding(1) var in_hdr_s: sampler;
@group(0) @binding(2) var in_bloom_t: texture_2d<f32>;
@group(0) @binding(3) var in_bloom_s: sampler;
@group(0) @binding(4) var<uniform> in_params: PostProcessParams;
@group(0) @binding(5) var<storage, read> in_exposure: ExposureState;

fn sample_hdr(uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(in_hdr_t, in_hdr_s, uv, 0.0).rgb;
}

// Dave Hoskins' hash without sine
fn hash13(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
}

fn interleaved_gradient_noise(pixel: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(dot(pixel, vec2<f32>(0.06711056, 0.00583715))));
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coord;
    let p = in_params;

    // Chromatic aberration
    var color: vec3<f32>;
    if p.effects.x > 0.0 {
        let offset = (uv - 0.5) * 2.0 * p.effects.x;
        color = vec3<f32>(sample_hdr(uv + offset).r, sample_hdr(uv).g, sample_hdr(uv - offset).b);
    } else {
        color = sample_hdr(uv);
    }

    // Bloom
    if p.bloom.w > 0.0 {
        color += textureSampleLevel(in_bloom_t, in_bloom_s, uv, 0.0).rgb * p.bloom.w;
    }

    // Exposure and color grading in HDR
    let adapted_ev = select(0.0, in_exposure.ev, p.exposure.x > 0.5 && in_exposure.initialized != 0u);
    color *= p.grading.x * exp2(adapted_ev);
    color = max(p.white_balance * color, vec3<f32>(0.0));
    color = 0.18 * pow(color / 0.18, vec3<f32>(p.grading.y));
    color *= p.color_filter.rgb;
    color = max(mix(vec3<f32>(luminance(color)), color, p.grading.z), vec3<f32>(0.0));

    color = tonemap(color, u32(p.grading.w));

    // Lift, gamma, gain
    color = p.gain.rgb * (color + p.lift.rgb * (1.0 - color));
    color = pow(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0 / p.gamma.rgb);

    // Vignette
    if p.vignette.x > 0.0 {
        let size = vec2<f32>(textureDimensions(in_hdr_t));
        var d = (uv - 0.5) * 2.0;
        d.y *= mix(1.0, size.y / size.x, p.vignette.w);
        let amount = smoothstep(p.vignette.y, p.vignette.y + p.vignette.z, length(d)) * p.vignette.x;
        color = mix(color, p.vignette_color.rgb, amount);
    }

    // Grain and dithering, in display space
    var display = pow(color, vec3<f32>(1.0 / 2.2));
    if p.effects.y > 0.0 {
        let noise = hash13(vec3<f32>(in.clip_position.xy, floor(p.time.x * 24.0))) - 0.5;
        let response = mix(1.0, 1.0 - luminance(display), p.effects.z);
        display += noise * p.effects.y * 0.25 * response;
    }
    if p.effects.w > 0.0 {
        display += (interleaved_gradient_noise(in.clip_position.xy) - 0.5) / 255.0;
    }
    color = pow(max(display, vec3<f32>(0.0)), vec3<f32>(2.2));

    return vec4<f32>(color, 1.0);
}
