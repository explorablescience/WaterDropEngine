// ================================================================
// Ambient lighting by the environment (see EnvironmentData).
// Include with:  #include "core/render/pbr/environment.wgsl"
// ================================================================
#include "core/render/pbr/pbr_functions.wgsl"

// L2 spherical harmonics of the diffuse light, already convolved with the cosine lobe and divided by π.
struct EnvironmentSH {
    coefficients: array<vec4<f32>, 9>
}

// Real spherical harmonics basis up to the band 2, for a unit direction.
fn sh_basis(d: vec3<f32>) -> array<f32, 9> {
    return array<f32, 9>(
        0.282095,
        0.488603 * d.y,
        0.488603 * d.z,
        0.488603 * d.x,
        1.092548 * d.x * d.y,
        1.092548 * d.y * d.z,
        0.315392 * (3.0 * d.z * d.z - 1.0),
        1.092548 * d.x * d.z,
        0.546274 * (d.x * d.x - d.y * d.y)
    );
}

// Diffuse light reaching a white surface of normal n.
fn environment_diffuse(sh: EnvironmentSH, n: vec3<f32>) -> vec3<f32> {
    let basis = sh_basis(n);
    var light = vec3<f32>(0.0);
    for (var i = 0u; i < 9u; i++) {
        light += sh.coefficients[i].rgb * basis[i];
    }
    // The truncated harmonics ring slightly negative opposite to bright lights
    return max(light, vec3<f32>(0.0));
}

// Occlusion of the specular light from the ambient occlusion (Lagarde and de Rousiers, 2014).
fn specular_occlusion(n_dot_v: f32, ao: f32, roughness: f32) -> f32 {
    let alpha = roughness * roughness;
    return clamp(pow(n_dot_v + ao, exp2(-16.0 * alpha - 1.0)) - 1.0 + ao, 0.0, 1.0);
}

struct EnvironmentLight {
    diffuse: vec3<f32>,
    specular: vec3<f32>
}

// Diffuse and specular light of a surface by the environment. The specular uses the split-sum approximation: the
// prefiltered environment along the reflection, times the reflectance of the BRDF lookup table.
fn environment_lighting(
    sh: EnvironmentSH,
    map: texture_cube<f32>,
    map_sampler: sampler,
    brdf_lut: texture_2d<f32>,
    n: vec3<f32>, v: vec3<f32>,
    albedo: vec3<f32>, metallic: f32, roughness: f32, f0: vec3<f32>, ao: f32
) -> EnvironmentLight {
    let n_dot_v = clamp(dot(n, v), 1e-4, 1.0);
    let brdf = textureSampleLevel(brdf_lut, map_sampler, vec2<f32>(n_dot_v, roughness), 0.0).rg;
    let reflectance = f0 * brdf.x + brdf.y;

    let lod = roughness * f32(textureNumLevels(map) - 1u);
    let prefiltered = textureSampleLevel(map, map_sampler, reflect(-v, n), lod).rgb;
    let specular = prefiltered * reflectance * specular_occlusion(n_dot_v, ao, roughness);

    // The light not reflected by the specular layer is diffused
    let diffuse = (1.0 - reflectance) * (1.0 - metallic) * albedo * environment_diffuse(sh, n) * ao;
    return EnvironmentLight(diffuse, specular);
}
