use bevy::prelude::*;

use crate::utils::Color;

/// Default color values for the lights.
const AMBIENT_DEFAULT:  f32 = 0.05;
const DIFFUSE_DEFAULT:  f32 = 0.8;
const SPECULAR_DEFAULT: f32 = 1.0;

/// A directional light is a light that emits light in a single direction from an infinite distance.
#[derive(Component, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct DirectionalLight {
    /// World space direction of the light.
    pub direction: Vec3,

    /// Ambient color of the light.
    pub ambient:  Color,
    /// Diffuse color of the light.
    pub diffuse:  Color,
    /// Specular color of the light.
    pub specular: Color
}
impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: Vec3::new(0.0, -1.0, 0.0),

            ambient:  Color::from_srgba(AMBIENT_DEFAULT,  AMBIENT_DEFAULT,  AMBIENT_DEFAULT,  1.0),
            diffuse:  Color::from_srgba(DIFFUSE_DEFAULT,  DIFFUSE_DEFAULT,  DIFFUSE_DEFAULT,  1.0),
            specular: Color::from_srgba(SPECULAR_DEFAULT, SPECULAR_DEFAULT, SPECULAR_DEFAULT, 1.0)
        }
    }
}

/// A point light is a light that emits light in all directions from a single point.
/// Use the `set_attenuation` method to set the attenuation factors of the light. By default, the
/// light has a full attenuation at 100 units.
#[derive(Component, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct PointLight {
    /// World space position of the light.
    pub position: Vec3,

    /// Ambient color of the light.
    pub ambient:  Color,
    /// Diffuse color of the light.
    pub diffuse:  Color,
    /// Specular color of the light.
    pub specular: Color,

    /// Constant attenuation factor of the light.
    pub constant:  f32,
    /// Linear attenuation factor of the light.
    pub linear:    f32,
    /// Quadratic attenuation factor of the light.
    pub quadratic: f32
}
impl PointLight {
    /// Sets the attenuation factors of the light.
    /// If the range is not a value in {7, 13, 20, 32, 50, 65, 100, 160, 200, 325, 600, 3250},
    /// the method will return None. Note that most of the light will be already attenuated at 20%.
    pub fn with_range(&mut self, range: f32) -> Option<Self> {
        match range {
            3250.0 => {
                self.constant  = 1.0;
                self.linear    = 0.0014;
                self.quadratic = 0.000007;
            },
            600.0 => {
                self.constant  = 1.0;
                self.linear    = 0.007;
                self.quadratic = 0.0002;
            },
            325.0 => {
                self.constant  = 1.0;
                self.linear    = 0.014;
                self.quadratic = 0.0007;
            },
            200.0 => {
                self.constant  = 1.0;
                self.linear    = 0.022;
                self.quadratic = 0.0019;
            },
            160.0 => {
                self.constant  = 1.0;
                self.linear    = 0.027;
                self.quadratic = 0.0028;
            },
            100.0 => {
                self.constant  = 1.0;
                self.linear    = 0.045;
                self.quadratic = 0.0075;
            },
            65.0 => {
                self.constant  = 1.0;
                self.linear    = 0.07;
                self.quadratic = 0.017;
            },
            50.0 => {
                self.constant  = 1.0;
                self.linear    = 0.09;
                self.quadratic = 0.032;
            },
            32.0 => {
                self.constant  = 1.0;
                self.linear    = 0.14;
                self.quadratic = 0.07;
            },
            20.0 => {
                self.constant  = 1.0;
                self.linear    = 0.22;
                self.quadratic = 0.20;
            },
            13.0 => {
                self.constant  = 1.0;
                self.linear    = 0.35;
                self.quadratic = 0.44;
            },
            7.0 => {
                self.constant  = 1.0;
                self.linear    = 0.7;
                self.quadratic = 1.8;
            },
            _ => return None,
        }
        Some(*self)
    }
}
impl Default for PointLight {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 0.0),

            ambient:  Color::from_srgba(AMBIENT_DEFAULT,  AMBIENT_DEFAULT,  AMBIENT_DEFAULT,  1.0),
            diffuse:  Color::from_srgba(DIFFUSE_DEFAULT,  DIFFUSE_DEFAULT,  DIFFUSE_DEFAULT,  1.0),
            specular: Color::from_srgba(SPECULAR_DEFAULT, SPECULAR_DEFAULT, SPECULAR_DEFAULT, 1.0),

            constant:  0.0,
            linear:    0.0,
            quadratic: 0.0
        }.with_range(100.0).unwrap()
    }
}

/// A spotlight is a point light with a direction and a cut-off angle.
/// Use the `set_attenuation` method to set the attenuation factors of the light. By default, the
/// light has a full attenuation at 50 units.
#[derive(Component, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct SpotLight {
    /// World space position of the light.
    pub position: Vec3,
    /// World space direction of the light.
    pub direction: Vec3,

    /// Ambient color of the light.
    pub ambient:  Color,
    /// Diffuse color of the light.
    pub diffuse:  Color,
    /// Specular color of the light.
    pub specular: Color,

    /// Constant attenuation factor of the light.
    pub constant:  f32,
    /// Linear attenuation factor of the light.
    pub linear:    f32,
    /// Quadratic attenuation factor of the light.
    pub quadratic: f32,

    /// Inner cut-off angle of the light (in degrees): the angle at which the light starts to decay.
    pub inner_cutoff:  f32,
    /// Outer cut-off angle of the light (in degrees): the angle at which the light is completely attenuated.
    pub outer_cutoff: f32
}
impl SpotLight {
    /// Sets the attenuation factors of the light.
    /// If the range is not a value in {7, 13, 20, 32, 50, 65, 100, 160, 200, 325, 600, 3250},
    /// the method will return None.
    pub fn with_range(&mut self, range: f32) -> Option<Self> {
        match range {
            3250.0 => {
                self.constant  = 1.0;
                self.linear    = 0.0014;
                self.quadratic = 0.000007;
            },
            600.0 => {
                self.constant  = 1.0;
                self.linear    = 0.007;
                self.quadratic = 0.0002;
            },
            325.0 => {
                self.constant  = 1.0;
                self.linear    = 0.014;
                self.quadratic = 0.0007;
            },
            200.0 => {
                self.constant  = 1.0;
                self.linear    = 0.022;
                self.quadratic = 0.0019;
            },
            160.0 => {
                self.constant  = 1.0;
                self.linear    = 0.027;
                self.quadratic = 0.0028;
            },
            100.0 => {
                self.constant  = 1.0;
                self.linear    = 0.045;
                self.quadratic = 0.0075;
            },
            65.0 => {
                self.constant  = 1.0;
                self.linear    = 0.07;
                self.quadratic = 0.017;
            },
            50.0 => {
                self.constant  = 1.0;
                self.linear    = 0.09;
                self.quadratic = 0.032;
            },
            32.0 => {
                self.constant  = 1.0;
                self.linear    = 0.14;
                self.quadratic = 0.07;
            },
            20.0 => {
                self.constant  = 1.0;
                self.linear    = 0.22;
                self.quadratic = 0.20;
            },
            13.0 => {
                self.constant  = 1.0;
                self.linear    = 0.35;
                self.quadratic = 0.44;
            },
            7.0 => {
                self.constant  = 1.0;
                self.linear    = 0.7;
                self.quadratic = 1.8;
            },
            _ => return None,
        }
        Some(*self)
    }
}
impl Default for SpotLight {
    fn default() -> Self {
        Self {
            position:  Vec3::new(0.0,  0.0, 0.0),
            direction: Vec3::new(0.0, -1.0, 0.0),

            ambient:  Color::from_srgba(AMBIENT_DEFAULT,  AMBIENT_DEFAULT,  AMBIENT_DEFAULT,  1.0),
            diffuse:  Color::from_srgba(DIFFUSE_DEFAULT,  DIFFUSE_DEFAULT,  DIFFUSE_DEFAULT,  1.0),
            specular: Color::from_srgba(SPECULAR_DEFAULT, SPECULAR_DEFAULT, SPECULAR_DEFAULT, 1.0),

            constant:  0.0,
            linear:    0.0,
            quadratic: 0.0,

            inner_cutoff: 0.0,
            outer_cutoff: std::f32::consts::PI / 4.0
        }.with_range(50.0).unwrap()
    }
}



/// Lights storage buffer
#[repr(C)]
#[derive(Resource, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightsStorageElement {
    /// World space position of the directional light for xyz. If it is the first element, the w component is the number of lights.
    pub position_number: [f32; 4],
    /// World space direction of the light. The w component is the type of the light: 0 for directional, 1 for point, 2 for spot.
    pub direction_type:  [f32; 4],
    /// Ambient color of the light. The w component is the constant attenuation factor if the light is a point light. It is the cos of the inner cut-off angle in radians if the light is a spot light.
    pub ambient_const:   [f32; 4],
    /// Diffuse color of the light. The w component is the linear attenuation factor if the light is a point light. It is the cos of the outer cut-off angle in radians if the light is a spot light.
    pub diffuse_linea:   [f32; 4],
    /// Specular color of the light. The w component is the quadratic attenuation factor if the light is a point light.
    pub specular_quadr:  [f32; 4],
    /// Inner and outer cut-off angles in radians if the light is a spot light.
    pub cut_off:         [f32; 4]
}
impl LightsStorageElement {
    pub fn from_directional(light: &DirectionalLight) -> Self {
        let ambient  = light.ambient.to_linear_rgba();
        let diffuse  = light.diffuse.to_linear_rgba();
        let specular = light.specular.to_linear_rgba();
        Self {
            position_number:    [0.0, 0.0, 0.0, 0.0],
            direction_type:     [light.direction.x, light.direction.y, light.direction.z, 0.0],
            ambient_const:      [ambient.r(),  ambient.g(),  ambient.b(),  0.0],
            diffuse_linea:      [diffuse.r(),  diffuse.g(),  diffuse.b(),  0.0],
            specular_quadr:     [specular.r(), specular.g(), specular.b(), 0.0],
            cut_off:            [0.0, 0.0, 0.0, 0.0]
        }
    }

    pub fn from_point(light: &PointLight) -> Self {
        let ambient  = light.ambient.to_linear_rgba();
        let diffuse  = light.diffuse.to_linear_rgba();
        let specular = light.specular.to_linear_rgba();
        Self {
            position_number:    [light.position.x, light.position.y, light.position.z, 0.0],
            direction_type:     [0.0, 0.0, 0.0, 1.0],
            ambient_const:      [ambient.r(),  ambient.g(),  ambient.b(),  light.constant],
            diffuse_linea:      [diffuse.r(),  diffuse.g(),  diffuse.b(),  light.linear],
            specular_quadr:     [specular.r(), specular.g(), specular.b(), light.quadratic],
            cut_off:            [0.0, 0.0, 0.0, 0.0]
        }
    }

    pub fn from_spot(light: &SpotLight) -> Self {
        let ambient  = light.ambient.to_linear_rgba();
        let diffuse  = light.diffuse.to_linear_rgba();
        let specular = light.specular.to_linear_rgba();
        Self {
            position_number:    [light.position.x,  light.position.y,  light.position.z,  0.0],
            direction_type:     [light.direction.x, light.direction.y, light.direction.z, 2.0],
            ambient_const:      [ambient.r(),  ambient.g(),  ambient.b(),  light.constant],
            diffuse_linea:      [diffuse.r(),  diffuse.g(),  diffuse.b(),  light.linear],
            specular_quadr:     [specular.r(), specular.g(), specular.b(), light.quadratic],
            cut_off:            [light.inner_cutoff.cos(), light.outer_cutoff.cos(), 0.0, 0.0]
        }
    }
}
