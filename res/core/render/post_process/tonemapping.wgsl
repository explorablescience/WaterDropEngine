// Tonemapping operators, from linear HDR to linear display colors in [0, 1].

fn tonemap_reinhard(color: vec3<f32>) -> vec3<f32> {
    return color / (1.0 + color);
}

// Stephen Hill's fit of the ACES RRT and ODT
fn tonemap_aces(color: vec3<f32>) -> vec3<f32> {
    let input = mat3x3<f32>(
        vec3<f32>(0.59719, 0.07600, 0.02840),
        vec3<f32>(0.35458, 0.90834, 0.13383),
        vec3<f32>(0.04823, 0.01566, 0.83777)
    );
    let output = mat3x3<f32>(
        vec3<f32>(1.60475, -0.10208, -0.00327),
        vec3<f32>(-0.53108, 1.10813, -0.07276),
        vec3<f32>(-0.07367, -0.00605, 1.07602)
    );
    let v = input * color;
    let fitted = (v * (v + 0.0245786) - 0.000090537) / (v * (0.983729 * v + 0.4329510) + 0.238081);
    return output * fitted;
}

// AgX with the default look (Benjamin Wrensch's approximation)
fn tonemap_agx(color: vec3<f32>) -> vec3<f32> {
    let agx_mat = mat3x3<f32>(
        0.842479062253094, 0.0423282422610123, 0.0423756549057051,
        0.0784335999999992, 0.878468636469772, 0.0784336,
        0.0792237451477643, 0.0791661274605434, 0.879142973793104
    );
    let agx_mat_inv = mat3x3<f32>(
        1.19687900512017, -0.0528968517574562, -0.0529716355144438,
        -0.0980208811401368, 1.15190312990417, -0.0980434501171241,
        -0.0990297440797205, -0.0989611768448433, 1.15107367264116
    );
    let min_ev = -12.47393;
    let max_ev = 4.026069;

    var v = agx_mat * color;
    v = clamp(log2(max(v, vec3<f32>(1e-10))), vec3<f32>(min_ev), vec3<f32>(max_ev));
    v = (v - min_ev) / (max_ev - min_ev);

    // Sigmoid contrast curve
    let v2 = v * v;
    let v4 = v2 * v2;
    v = 15.5 * v4 * v2 - 40.14 * v4 * v + 31.96 * v4 - 6.868 * v2 * v + 0.4298 * v2 + 0.1191 * v - 0.00232;

    v = agx_mat_inv * v;
    return pow(max(v, vec3<f32>(0.0)), vec3<f32>(2.2));
}

// Khronos PBR Neutral
fn tonemap_pbr_neutral(color_in: vec3<f32>) -> vec3<f32> {
    let start_compression = 0.8 - 0.04;
    let desaturation = 0.15;

    var color = color_in;
    let x = min(color.r, min(color.g, color.b));
    let offset = select(0.04, x - 6.25 * x * x, x < 0.08);
    color -= offset;

    let peak = max(color.r, max(color.g, color.b));
    if peak < start_compression {
        return color;
    }
    let d = 1.0 - start_compression;
    let new_peak = 1.0 - d * d / (peak + d - start_compression);
    color *= new_peak / peak;
    let g = 1.0 - 1.0 / (desaturation * (peak - new_peak) + 1.0);
    return mix(color, vec3<f32>(new_peak), g);
}

// Modes: 0 = none, 1 = Reinhard, 2 = ACES, 3 = AgX, 4 = PBR Neutral
fn tonemap(color: vec3<f32>, mode: u32) -> vec3<f32> {
    var mapped = color;
    switch mode {
        case 1u: { mapped = tonemap_reinhard(color); }
        case 2u: { mapped = tonemap_aces(color); }
        case 3u: { mapped = tonemap_agx(color); }
        case 4u: { mapped = tonemap_pbr_neutral(color); }
        default: {}
    }
    return clamp(mapped, vec3<f32>(0.0), vec3<f32>(1.0));
}
