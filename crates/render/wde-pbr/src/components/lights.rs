use crate::prelude::Color as WdeColor;
use bevy::prelude::*;

/// Default values for the lights.
const DEFAULT_INTENSITY: f32 = 1.0;
const INTENSITY_MULTIPLIER: f32 = 10.0; // To convert from "standard" intensity to lumens

/// A directional light is a light that emits light in a single direction from an infinite distance.
#[derive(Component, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct DirectionalLight {
    /// World space direction of the light.
    pub direction: Vec3,

    /// Light color (RGB).
    pub color: WdeColor,
    /// Light intensity/brightness (in lumens, physical-based).
    pub intensity: f32
}
impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: Vec3::new(0.0, -1.0, 0.0),
            color: WdeColor::from_srgba(1.0, 1.0, 1.0, 1.0),
            intensity: DEFAULT_INTENSITY * INTENSITY_MULTIPLIER
        }
    }
}

/// A point light is a light that emits light in all directions from a single point.
#[derive(Component, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct PointLight {
    /// World space position of the light.
    pub position: Vec3,

    /// Light color (RGB).
    pub color: WdeColor,
    /// Light intensity/brightness (in lumens, physical-based).
    pub intensity: f32,
    /// Maximum range of light influence (in world units).
    pub range: f32
}
impl Default for PointLight {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 0.0),
            color: WdeColor::from_srgba(1.0, 1.0, 1.0, 1.0),
            intensity: DEFAULT_INTENSITY * INTENSITY_MULTIPLIER,
            range: 100.0
        }
    }
}

/// A spotlight is a point light with a direction and cone angles.
#[derive(Component, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct SpotLight {
    /// World space position of the light.
    pub position: Vec3,
    /// World space direction of the light.
    pub direction: Vec3,

    /// Light color (RGB).
    pub color: WdeColor,
    /// Light intensity/brightness (in lumens, physical-based).
    pub intensity: f32,
    /// Maximum range of light influence (in world units).
    pub range: f32,

    /// Inner cone angle of the light (in radians): full brightness cone.
    pub inner_cone: f32,
    /// Outer cone angle of the light (in radians): edge of cone with smooth falloff.
    pub outer_cone: f32
}
impl Default for SpotLight {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 0.0),
            direction: Vec3::new(0.0, -1.0, 0.0),
            color: WdeColor::from_srgba(1.0, 1.0, 1.0, 1.0),
            intensity: DEFAULT_INTENSITY * INTENSITY_MULTIPLIER,
            range: 50.0,
            inner_cone: std::f32::consts::PI / 12.0, // 15 degrees
            outer_cone: std::f32::consts::PI / 6.0   // 30 degrees
        }
    }
}

/// Lights storage buffer (matches the shader Light struct)
#[repr(C)]
#[derive(Resource, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct LightsStorageElement {
    /// (x, y, z) = World space position of the light - w = Light intensity (brightness in lumens)
    pub position_intensity: [f32; 4],
    /// (x, y, z) = Light color - w = Light type (0 = directional, 1 = point, 2 = spot)
    pub color_type: [f32; 4],
    /// (x, y, z) = Direction of the light - w = Light range (max influence distance)
    pub direction_range: [f32; 4],
    /// Spot light inner and outer cone angles (cos values). Only used for spot lights.
    pub spot_cone: [f32; 4]
}
impl LightsStorageElement {
    pub fn from_directional(light: &DirectionalLight) -> Self {
        let color = light.color.to_linear_rgba();
        Self {
            position_intensity: [0.0, 0.0, 0.0, light.intensity * INTENSITY_MULTIPLIER],
            color_type: [color.r(), color.g(), color.b(), 0.0],
            direction_range: [light.direction.x, light.direction.y, light.direction.z, 0.0],
            spot_cone: [0.0, 0.0, 0.0, 0.0]
        }
    }

    pub fn from_point(light: &PointLight) -> Self {
        let color = light.color.to_linear_rgba();
        Self {
            position_intensity: [
                light.position.x,
                light.position.y,
                light.position.z,
                light.intensity * INTENSITY_MULTIPLIER
            ],
            color_type: [color.r(), color.g(), color.b(), 1.0],
            direction_range: [0.0, 0.0, 0.0, light.range],
            spot_cone: [0.0, 0.0, 0.0, 0.0]
        }
    }

    pub fn from_spot(light: &SpotLight) -> Self {
        let color = light.color.to_linear_rgba();
        Self {
            position_intensity: [
                light.position.x,
                light.position.y,
                light.position.z,
                light.intensity * INTENSITY_MULTIPLIER
            ],
            color_type: [color.r(), color.g(), color.b(), 2.0],
            direction_range: [
                light.direction.x,
                light.direction.y,
                light.direction.z,
                light.range
            ],
            spot_cone: [light.inner_cone.cos(), light.outer_cone.cos(), 0.0, 0.0]
        }
    }
}
