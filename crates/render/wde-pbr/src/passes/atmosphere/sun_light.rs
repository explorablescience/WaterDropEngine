use bevy::prelude::*;
use wde_renderer::prelude::Color;

use crate::prelude::*;

const TAU: f32 = std::f32::consts::PI * 2.0;
const SUN_AZIMUTH: f32 = -0.95;

/// Marker component for the atmosphere-driven sun light.
#[derive(Component, Default)]
pub struct AtmosphereSunLight;

pub struct AtmosphereSunLightPlugin;
impl Plugin for AtmosphereSunLightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (ensure_sun_light, update_sun_light));
    }
}

fn ensure_sun_light(mut commands: Commands, sun_query: Query<Entity, With<AtmosphereSunLight>>) {
    if sun_query.is_empty() {
        commands.spawn((DirectionalLight::default(), AtmosphereSunLight));
    }
}

fn update_sun_light(
    time: Res<Time>,
    atmosphere: Res<AtmosphereSettings>,
    mut lights: Query<&mut DirectionalLight, With<AtmosphereSunLight>>
) {
    let day_length = atmosphere.day_length_seconds.max(1.0);
    let time_of_day = if atmosphere.animate_time {
        (time.elapsed_secs() / day_length).fract()
    } else {
        atmosphere.time_of_day.fract()
    };

    let sun_elevation = (time_of_day - 0.25) * TAU;
    // Direction toward the sun — matches the atmosphere shader exactly.
    let sun_dir = Vec3::new(
        sun_elevation.cos() * SUN_AZIMUTH.cos(),
        sun_elevation.sin(),
        sun_elevation.cos() * SUN_AZIMUTH.sin()
    )
    .normalize();

    // Match the same transition logic used by the atmosphere shader.
    let twilight_w = 0.06 + (0.20 - 0.06) * atmosphere.twilight_softness.clamp(0.0, 1.0);
    let day = smoothstep(-twilight_w, twilight_w, sun_dir.y);
    let sun_visible = smoothstep(
        atmosphere.sun_visibility_min,
        atmosphere.sun_visibility_max,
        sun_dir.y
    );

    let sun_color = Vec3::new(
        atmosphere.sun_day_color[0] * (1.0 - day) + atmosphere.sun_night_color[0] * day,
        atmosphere.sun_day_color[1] * (1.0 - day) + atmosphere.sun_night_color[1] * day,
        atmosphere.sun_day_color[2] * (1.0 - day) + atmosphere.sun_night_color[2] * day
    );

    for mut light in &mut lights {
        // DirectionalLight.direction is the ray travel direction (sun → scene),
        // which is the opposite of the toward-sun vector.
        light.direction = -sun_dir;
        light.color = Color::from_srgba(sun_color.x, sun_color.y, sun_color.z, 1.0);
        light.intensity = (atmosphere.sun_intensity * sun_visible).max(0.0);
    }
}

#[inline]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if edge0 == edge1 {
        return if x < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
