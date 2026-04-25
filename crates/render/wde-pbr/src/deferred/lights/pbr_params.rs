use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};

#[derive(Resource, Clone, Copy, Debug, Reflect)]
#[reflect(Resource)]
pub struct PbrSettings {
    pub direct_scale: f32,
    pub ambient_scale: f32,
    pub exposure: f32,
    pub tone_map_white: f32,
    pub metallic_scale: f32,
    pub roughness_scale: f32,
    pub min_roughness: f32
}
impl Default for PbrSettings {
    fn default() -> Self {
        Self {
            direct_scale: 1.0,
            ambient_scale: 0.2,
            exposure: 1.0,
            tone_map_white: 1.0,
            metallic_scale: 1.0,
            roughness_scale: 1.0,
            min_roughness: 0.0
        }
    }
}

#[repr(C)]
#[derive(Resource, Clone, Copy, Debug, Pod, Zeroable)]
pub struct PbrParamsUniform {
    /// x=direct_scale, y=ambient_scale, z=exposure, w=tone_map_white
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
                settings.tone_map_white.max(0.001)
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
