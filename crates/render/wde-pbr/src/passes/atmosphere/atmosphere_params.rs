use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};

/// Runtime settings controlling the atmosphere model.
#[derive(Resource, Clone, Copy, Debug, Reflect)]
#[reflect(Resource)]
pub struct AtmosphereSettings {
    /// Manual normalized time of day in [0, 1].
    pub time_of_day: f32,
    /// If true, time_of_day is animated using day_length_seconds.
    pub animate_time: bool,
    /// Full day length in seconds used to animate time_of_day.
    pub day_length_seconds: f32,

    pub sun_intensity: f32,
    pub twilight_softness: f32,
    pub star_intensity: f32,

    pub day_horizon_exp: f32,
    pub night_horizon_exp: f32,
    pub twilight_horizon_exp: f32,
    pub twilight_intensity: f32,

    pub sun_visibility_min: f32,
    pub sun_visibility_max: f32,
    pub sun_disk_min: f32,
    pub sun_disk_max: f32,
    pub sun_corona_exp: f32,
    pub sun_corona_intensity: f32,

    pub star_density_threshold: f32,
    pub star_core_scale: f32,
    pub star_min_altitude: f32,
    pub star_max_altitude: f32,

    pub ground_falloff: f32,

    pub day_zenith_color: [f32; 3],
    pub day_horizon_color: [f32; 3],
    pub night_zenith_color: [f32; 3],
    pub night_horizon_color: [f32; 3],
    pub twilight_tint_color: [f32; 3],
    pub sun_day_color: [f32; 3],
    pub sun_night_color: [f32; 3],
    pub star_color: [f32; 3],
    pub ground_night_color: [f32; 3],
    pub ground_day_color: [f32; 3]
}
impl Default for AtmosphereSettings {
    fn default() -> Self {
        Self {
            time_of_day: 0.3,
            animate_time: !cfg!(debug_assertions),
            day_length_seconds: 180.0,
            sun_intensity: 1.0,
            twilight_softness: 0.6,
            star_intensity: 0.75,
            day_horizon_exp: 0.52,
            night_horizon_exp: 0.78,
            twilight_horizon_exp: 1.25,
            twilight_intensity: 0.45,
            sun_visibility_min: -0.14,
            sun_visibility_max: 0.03,
            sun_disk_min: 0.9992,
            sun_disk_max: 0.99992,
            sun_corona_exp: 22.0,
            sun_corona_intensity: 0.7,
            star_density_threshold: 0.9992,
            star_core_scale: 1250.0,
            star_min_altitude: 0.08,
            star_max_altitude: 0.30,
            ground_falloff: 3.5,
            day_zenith_color: [0.25, 0.10, 0.05],
            day_horizon_color: [0.95, 0.48, 0.20],
            night_zenith_color: [0.008, 0.012, 0.025],
            night_horizon_color: [0.028, 0.022, 0.030],
            twilight_tint_color: [1.15, 0.36, 0.10],
            sun_day_color: [1.35, 0.70, 0.28],
            sun_night_color: [1.06, 0.95, 0.86],
            star_color: [0.67, 0.73, 0.92],
            ground_night_color: [0.015, 0.010, 0.012],
            ground_day_color: [0.085, 0.036, 0.016]
        }
    }
}

#[repr(C)]
#[derive(Resource, Clone, Copy, Debug, Pod, Zeroable)]
pub struct AtmosphereParamsUniform {
    pub time_params: [f32; 4],
    pub gradient_params: [f32; 4],
    pub sun_params: [f32; 4],
    pub sun_star_params: [f32; 4],
    pub star_ground_params: [f32; 4],
    pub day_zenith_color: [f32; 4],
    pub day_horizon_color: [f32; 4],
    pub night_zenith_color: [f32; 4],
    pub night_horizon_color: [f32; 4],
    pub twilight_tint_color: [f32; 4],
    pub sun_day_color: [f32; 4],
    pub sun_night_color: [f32; 4],
    pub star_color: [f32; 4],
    pub ground_night_color: [f32; 4],
    pub ground_day_color: [f32; 4]
}
impl Default for AtmosphereParamsUniform {
    fn default() -> Self {
        Self::from_settings(&AtmosphereSettings::default(), 0.5)
    }
}

impl AtmosphereParamsUniform {
    #[inline]
    pub fn from_settings(settings: &AtmosphereSettings, time_of_day: f32) -> Self {
        Self {
            time_params: [
                time_of_day,
                settings.sun_intensity,
                settings.twilight_softness,
                settings.star_intensity
            ],
            gradient_params: [
                settings.day_horizon_exp,
                settings.night_horizon_exp,
                settings.twilight_horizon_exp,
                settings.twilight_intensity
            ],
            sun_params: [
                settings.sun_visibility_min,
                settings.sun_visibility_max,
                settings.sun_disk_min,
                settings.sun_disk_max
            ],
            sun_star_params: [
                settings.sun_corona_exp,
                settings.sun_corona_intensity,
                settings.star_density_threshold,
                settings.star_core_scale
            ],
            star_ground_params: [
                settings.star_min_altitude,
                settings.star_max_altitude,
                settings.ground_falloff,
                0.0
            ],
            day_zenith_color: [
                settings.day_zenith_color[0],
                settings.day_zenith_color[1],
                settings.day_zenith_color[2],
                1.0
            ],
            day_horizon_color: [
                settings.day_horizon_color[0],
                settings.day_horizon_color[1],
                settings.day_horizon_color[2],
                1.0
            ],
            night_zenith_color: [
                settings.night_zenith_color[0],
                settings.night_zenith_color[1],
                settings.night_zenith_color[2],
                1.0
            ],
            night_horizon_color: [
                settings.night_horizon_color[0],
                settings.night_horizon_color[1],
                settings.night_horizon_color[2],
                1.0
            ],
            twilight_tint_color: [
                settings.twilight_tint_color[0],
                settings.twilight_tint_color[1],
                settings.twilight_tint_color[2],
                1.0
            ],
            sun_day_color: [
                settings.sun_day_color[0],
                settings.sun_day_color[1],
                settings.sun_day_color[2],
                1.0
            ],
            sun_night_color: [
                settings.sun_night_color[0],
                settings.sun_night_color[1],
                settings.sun_night_color[2],
                1.0
            ],
            star_color: [settings.star_color[0], settings.star_color[1], settings.star_color[2], 1.0],
            ground_night_color: [
                settings.ground_night_color[0],
                settings.ground_night_color[1],
                settings.ground_night_color[2],
                1.0
            ],
            ground_day_color: [
                settings.ground_day_color[0],
                settings.ground_day_color[1],
                settings.ground_day_color[2],
                1.0
            ]
        }
    }
}
