use bevy::{ecs::system::SystemParamItem, math::Vec3, prelude::*};
use wde_renderer::prelude::*;

/// Size in pixels of the faces of the environment map.
pub const ENVIRONMENT_MAP_SIZE: u32 = 128;
/// Number of mips of the environment map. Mip `i` is prefiltered for the roughness `i / (ENVIRONMENT_MAP_MIPS - 1)`.
pub const ENVIRONMENT_MAP_MIPS: u32 = 7;
pub const ENVIRONMENT_MAP_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

const BRDF_LUT_SIZE: u32 = 64;
const BRDF_LUT_SAMPLES: u32 = 128;

/// Ambient lighting of the scene by its environment (the sky), sampled by the lighting shaders:
/// - The diffuse irradiance, as 9 RGB spherical harmonics coefficients (`vec4`s) already convolved with the cosine lobe
///   and divided by π, so that they evaluate to the diffuse light of a white surface.
/// - The specular radiance, as a cube map whose mips are prefiltered for increasing roughnesses (GGX).
/// - The scale and bias of the specular reflectance in the split-sum approximation, as a 2D lookup table of
///   (n·v, roughness).
///
/// It is black until it is baked, by the atmosphere when the feature `atmosphere` is enabled.
#[derive(Asset, Clone, TypePath, Default)]
pub struct EnvironmentData;
impl EnvironmentData {
    pub const SH_BUFFER_IDX: u32 = 0;
    pub const MAP_IDX: u32 = 1;
    pub const BRDF_LUT_IDX: u32 = 2;

    pub const SH_COEFFICIENTS: usize = 9;

    /// Adds the environment to a lighting binding: the spherical harmonics, the environment map and its sampler, then the
    /// BRDF lookup table (sampled with the same sampler).
    pub fn add_bindings(
        builder: &mut RenderBindingBuilder,
        environment: &RenderAssets<GpuRenderData<EnvironmentData>>
    ) {
        builder
            .add_buffer(environment, Self::SH_BUFFER_IDX)
            .add_texture_cube_view(environment, Self::MAP_IDX)
            .add_texture_sampler(environment, Self::MAP_IDX)
            .add_texture_view(environment, Self::BRDF_LUT_IDX);
    }
}
impl RenderData for EnvironmentData {
    type Params = ();

    fn describe(_: &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        let sh_size = Self::SH_COEFFICIENTS * std::mem::size_of::<[f32; 4]>();
        builder
            .add_buffer(
                Self::SH_BUFFER_IDX,
                Buffer {
                    label: "environment-sh".to_string(),
                    size: sh_size,
                    usage: BufferUsage::UNIFORM | BufferUsage::STORAGE | BufferUsage::COPY_DST,
                    content: Some(vec![0; sh_size])
                }
            )
            .add_texture(
                Self::MAP_IDX,
                Texture {
                    label: "environment-map".to_string(),
                    size: (ENVIRONMENT_MAP_SIZE, ENVIRONMENT_MAP_SIZE),
                    format: ENVIRONMENT_MAP_FORMAT,
                    usages: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
                    layer_count: 6,
                    mip_level_count: ENVIRONMENT_MAP_MIPS,
                    cube: true,
                    ..Default::default()
                }
            )
            .add_texture(
                Self::BRDF_LUT_IDX,
                Texture {
                    label: "environment-brdf-lut".to_string(),
                    size: (BRDF_LUT_SIZE, BRDF_LUT_SIZE),
                    format: TextureFormat::Rg16Float,
                    usages: TextureUsages::TEXTURE_BINDING,
                    data: brdf_lut(),
                    ..Default::default()
                }
            );
    }
}

/// Scale and bias of F0 in the split-sum specular reflectance, for (n·v, roughness) at the texel centers.
fn brdf_lut() -> Vec<u8> {
    let mut data = Vec::with_capacity((BRDF_LUT_SIZE * BRDF_LUT_SIZE * 4) as usize);
    for y in 0..BRDF_LUT_SIZE {
        let roughness = (y as f32 + 0.5) / BRDF_LUT_SIZE as f32;
        for x in 0..BRDF_LUT_SIZE {
            let n_dot_v = (x as f32 + 0.5) / BRDF_LUT_SIZE as f32;
            let (scale, bias) = integrate_brdf(n_dot_v, roughness);
            data.extend_from_slice(&half::f16::from_f32(scale).to_bits().to_le_bytes());
            data.extend_from_slice(&half::f16::from_f32(bias).to_bits().to_le_bytes());
        }
    }
    data
}

/// Integrates the GGX specular BRDF over the hemisphere (normal +Z), with importance sampling.
fn integrate_brdf(n_dot_v: f32, roughness: f32) -> (f32, f32) {
    let v = Vec3::new((1.0 - n_dot_v * n_dot_v).sqrt(), 0.0, n_dot_v);
    let alpha = roughness * roughness;
    // Schlick-GGX remapping for image based lighting
    let k = alpha / 2.0;
    let g1 = |n_dot_x: f32| n_dot_x / (n_dot_x * (1.0 - k) + k);

    let (mut scale, mut bias) = (0.0, 0.0);
    for i in 0..BRDF_LUT_SAMPLES {
        let xi = (
            i as f32 / BRDF_LUT_SAMPLES as f32,
            i.reverse_bits() as f32 / 4_294_967_296.0
        );
        let phi = std::f32::consts::TAU * xi.0;
        let cos_theta = ((1.0 - xi.1) / (1.0 + (alpha * alpha - 1.0) * xi.1)).sqrt();
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
        let h = Vec3::new(phi.cos() * sin_theta, phi.sin() * sin_theta, cos_theta);
        let l = 2.0 * v.dot(h) * h - v;

        let n_dot_l = l.z;
        if n_dot_l > 0.0 {
            let v_dot_h = v.dot(h).max(0.0);
            let visibility = g1(n_dot_v) * g1(n_dot_l) * v_dot_h / (h.z * n_dot_v);
            let fresnel = (1.0 - v_dot_h).powi(5);
            scale += (1.0 - fresnel) * visibility;
            bias += fresnel * visibility;
        }
    }
    (
        scale / BRDF_LUT_SAMPLES as f32,
        bias / BRDF_LUT_SAMPLES as f32
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brdf_lut_matches_known_values() {
        // Smooth surfaces seen from the front reflect F0, rough ones lose energy to the masking
        let (scale, bias) = integrate_brdf(1.0, 0.05);
        assert!((scale - 1.0).abs() < 0.02 && bias < 0.01, "{scale} {bias}");
        let (scale, bias) = integrate_brdf(0.5, 1.0);
        assert!(scale + bias < 0.8, "{scale} {bias}");
        // Grazing angles reflect more, through the bias
        assert!(integrate_brdf(0.05, 0.3).1 > integrate_brdf(0.9, 0.3).1);
    }
}
