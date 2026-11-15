use bevy::prelude::*;
use wde_renderer::prelude::WdeColor;

use super::atmosphere_params::smoothstep;
use crate::prelude::*;

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
    let sun_dir =
        AtmosphereSettings::sun_direction(atmosphere.current_time_of_day(time.elapsed_secs()));
    let day = atmosphere.day_amount(sun_dir.y);
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
        light.color = WdeColor::from_srgba(sun_color.x, sun_color.y, sun_color.z, 1.0);
        light.intensity = (atmosphere.sun_intensity * sun_visible).max(0.0);
    }
}
