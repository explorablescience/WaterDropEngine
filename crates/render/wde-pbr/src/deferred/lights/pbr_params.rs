use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};

#[derive(Resource, Clone, Copy, Debug, Reflect)]
#[reflect(Resource)]
pub struct PbrSettings {
    pub direct_scale: f32,
    /// Scale of the ambient light of the environment (diffuse and specular), see [`crate::prelude::EnvironmentData`].
    /// At 1, it is the light of the sky as rendered.
    pub ambient_scale: f32,
    /// Scale of the lit color. The whole image is exposed in [`crate::post_process::ColorGrading`].
    pub exposure: f32,
    pub metallic_scale: f32,
    pub roughness_scale: f32,
    pub min_roughness: f32,
    pub debug_view: PbrDebugView
}
impl Default for PbrSettings {
    fn default() -> Self {
        Self {
            direct_scale: 1.0,
            ambient_scale: 0.2,
            exposure: 1.0,
            metallic_scale: 1.0,
            roughness_scale: 1.0,
            min_roughness: 0.0,
            debug_view: PbrDebugView::Off
        }
    }
}

/// What the PBR lighting shows instead of the lit surfaces. The sky is black in every view.
///
/// The lighting views show one term of the light, at the exposure of the last lit frame and with the color grading,
/// but without the fog, the bloom and the screen effects. The data views show a surface property as it is, without
/// any post-processing. Same values as the `PBR_DEBUG_*` constants of `pbr_params.wgsl`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum PbrDebugView {
    #[default]
    Off = 0,
    /// Every surface grey and non-metallic, keeping its normals and roughness.
    Clay = 1,
    /// Light of the light sources, shadowed.
    Direct = 2,
    AmbientDiffuse = 3,
    AmbientSpecular = 4,
    Emissive = 5,
    /// Lit surfaces tinted by shadow cascade, striped where two cascades are blended (see
    /// [`SHADOW_CASCADE_COLORS`](crate::prelude::SHADOW_CASCADE_COLORS)).
    ShadowCascades = 6,
    /// Linear albedo.
    Albedo = 7,
    /// World normals, from [-1, 1] to [0, 1].
    Normals = 8,
    /// Roughness, after the [`PbrSettings::roughness_scale`].
    Roughness = 9,
    /// Metallic, after the [`PbrSettings::metallic_scale`].
    Metallic = 10,
    /// Occlusion of the ambient light: of the material, times the screen-space one.
    Occlusion = 11,
    /// Visibility of the directional light through the shadow maps.
    Shadow = 12,
    /// Shadow map texels per screen pixel: red under 1/8, orange 1/4, yellow 1/2, green 1, blue over 2.
    ShadowTexels = 13
}
impl PbrDebugView {
    pub const ALL: [Self; 14] = [
        Self::Off,
        Self::Clay,
        Self::Direct,
        Self::AmbientDiffuse,
        Self::AmbientSpecular,
        Self::Emissive,
        Self::ShadowCascades,
        Self::Albedo,
        Self::Normals,
        Self::Roughness,
        Self::Metallic,
        Self::Occlusion,
        Self::Shadow,
        Self::ShadowTexels
    ];

    /// Whether it shows a surface property rather than light.
    pub fn is_data(self) -> bool {
        self as u32 >= Self::Albedo as u32
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Clay => "Clay",
            Self::Direct => "Direct Light",
            Self::AmbientDiffuse => "Ambient Diffuse",
            Self::AmbientSpecular => "Ambient Specular",
            Self::Emissive => "Emissive",
            Self::Albedo => "Albedo",
            Self::Normals => "Normals",
            Self::Roughness => "Roughness",
            Self::Metallic => "Metallic",
            Self::Occlusion => "Occlusion",
            Self::Shadow => "Shadow",
            Self::ShadowCascades => "Shadow Cascades",
            Self::ShadowTexels => "Shadow Texels"
        }
    }
}

#[repr(C)]
#[derive(Resource, Clone, Copy, Debug, Pod, Zeroable)]
pub struct PbrParamsUniform {
    /// x=direct_scale, y=ambient_scale, z=exposure, w=debug_view
    pub lighting_params: [f32; 4],
    /// x=metallic_scale, y=roughness_scale, z=min_roughness
    pub material_params: [f32; 4]
}
impl Default for PbrParamsUniform {
    fn default() -> Self {
        Self::from_settings(&PbrSettings::default())
    }
}

impl PbrParamsUniform {
    #[inline]
    pub fn from_settings(settings: &PbrSettings) -> Self {
        Self {
            lighting_params: [
                settings.direct_scale,
                settings.ambient_scale,
                settings.exposure,
                settings.debug_view as u32 as f32
            ],
            material_params: [
                settings.metallic_scale,
                settings.roughness_scale,
                settings.min_roughness.clamp(0.001, 1.0),
                0.0
            ]
        }
    }
}
