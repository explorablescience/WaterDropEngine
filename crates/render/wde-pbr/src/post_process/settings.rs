use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};

/// Settings of the post-processing stack. Insert it in the main world to configure the effects.
///
/// Every color is in linear RGB.
#[derive(Resource, Clone, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct PostProcessSettings {
    pub fog: FogSettings,
    pub bloom: BloomSettings,
    pub color_grading: ColorGrading,
    pub tonemapping: Tonemapping,
    pub vignette: VignetteSettings,
    pub chromatic_aberration: ChromaticAberrationSettings,
    pub film_grain: FilmGrainSettings,
    /// Adds a tiny noise to the output to hide the banding of 8-bit gradients.
    pub dithering: bool,
    pub anti_aliasing: AntiAliasing
}

/// Anti-aliasing of the final image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum AntiAliasing {
    None,
    /// Fast approximate anti-aliasing: blurs the contrasted edges along their direction.
    #[default]
    Fxaa
}

/// Exponential height fog, applied to the opaque geometry and to the sky, before the transparent objects.
#[derive(Clone, Debug, Reflect)]
pub struct FogSettings {
    pub enabled: bool,
    pub color: [f32; 3],
    /// Extinction per meter at the base height.
    pub density: f32,
    /// How fast the density decreases with the height above the base height. 0 gives a uniform fog.
    pub height_falloff: f32,
    pub base_height: f32,
    /// Distance from the camera before which there is no fog.
    pub start_distance: f32,
    /// Maximum fog opacity in [0, 1].
    pub max_opacity: f32,
    /// Distance at which the sky is fogged.
    pub sky_distance: f32,
    /// How much the fog takes the color of the sky horizon, in [0, 1] (requires the `atmosphere` feature).
    pub sky_color: f32,
    /// Color of the light scattered by the fog towards the directional light.
    pub sun_color: [f32; 3],
    /// Strength of the scattered directional light. 0 disables it.
    pub sun_intensity: f32,
    /// Concentration of the scattered light around the light direction.
    pub sun_exponent: f32
}
impl Default for FogSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            color: [0.5, 0.6, 0.7],
            density: 0.002,
            height_falloff: 0.05,
            base_height: 0.0,
            start_distance: 0.0,
            max_opacity: 1.0,
            sky_distance: 1000.0,
            sky_color: 0.0,
            sun_color: [1.0, 0.9, 0.7],
            sun_intensity: 0.0,
            sun_exponent: 8.0
        }
    }
}

/// Bloom of the bright parts of the image.
#[derive(Clone, Debug, Reflect)]
pub struct BloomSettings {
    pub enabled: bool,
    /// Strength of the bloom added to the image.
    pub intensity: f32,
    /// Brightness from which pixels bloom. 0 makes the whole image bloom.
    pub threshold: f32,
    /// Smoothness of the threshold transition, in [0, 1].
    pub threshold_softness: f32,
    /// Spread of the bloom, in [0, 1].
    pub scatter: f32
}
impl Default for BloomSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 0.1,
            threshold: 1.0,
            threshold_softness: 0.5,
            scatter: 0.7
        }
    }
}

/// Color grading, applied in HDR before the tonemapping, except `lift`, `gamma` and `gain` which are applied after.
#[derive(Clone, Debug, Reflect)]
pub struct ColorGrading {
    /// Exposure in EV stops.
    pub exposure: f32,
    /// White balance temperature in [-1, 1]: negative is cooler, positive warmer.
    pub temperature: f32,
    /// White balance tint in [-1, 1]: negative is greener, positive more magenta.
    pub tint: f32,
    /// Contrast around the mid-grey. 1 is neutral.
    pub contrast: f32,
    /// 0 is greyscale, 1 is neutral.
    pub saturation: f32,
    /// Multiplier of the image color.
    pub color_filter: [f32; 3],
    /// Shadows offset, 0 is neutral.
    pub lift: [f32; 3],
    /// Midtones power, 1 is neutral.
    pub gamma: [f32; 3],
    /// Highlights multiplier, 1 is neutral.
    pub gain: [f32; 3]
}
impl Default for ColorGrading {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            temperature: 0.0,
            tint: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            color_filter: [1.0; 3],
            lift: [0.0; 3],
            gamma: [1.0; 3],
            gain: [1.0; 3]
        }
    }
}

/// Operator mapping the HDR colors to the display range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum Tonemapping {
    /// Clamps the colors.
    None,
    Reinhard,
    /// Stephen Hill's fit of the ACES curve. Contrasted, shifts saturated colors.
    Aces,
    /// Desaturates the highlights smoothly.
    AgX,
    /// Khronos PBR Neutral: keeps the colors below 0.76 untouched and only compresses the highlights.
    #[default]
    PbrNeutral
}

/// Darkening of the image borders.
#[derive(Clone, Debug, Reflect)]
pub struct VignetteSettings {
    /// 0 disables the vignette.
    pub intensity: f32,
    /// Distance from the center where the vignette starts, 1 being the middle of the left and right borders.
    pub radius: f32,
    /// Width of the vignette transition.
    pub smoothness: f32,
    /// 0 follows the screen aspect ratio, 1 is a circle.
    pub roundness: f32,
    pub color: [f32; 3]
}
impl Default for VignetteSettings {
    fn default() -> Self {
        Self {
            intensity: 0.0,
            radius: 0.6,
            smoothness: 0.8,
            roundness: 1.0,
            color: [0.0; 3]
        }
    }
}

/// Splitting of the color channels towards the image borders.
#[derive(Clone, Debug, Default, Reflect)]
pub struct ChromaticAberrationSettings {
    /// Channel offset at the image corners, in percent of the screen. 0 disables it.
    pub intensity: f32
}

/// Animated film grain noise.
#[derive(Clone, Debug, Reflect)]
pub struct FilmGrainSettings {
    /// 0 disables the grain.
    pub intensity: f32,
    /// How much the grain is reduced in the bright areas, in [0, 1].
    pub response: f32
}
impl Default for FilmGrainSettings {
    fn default() -> Self {
        Self {
            intensity: 0.0,
            response: 0.8
        }
    }
}

/// GPU layout of the post-processing parameters (bloom and final composite).
#[repr(C)]
#[derive(Resource, Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct PostProcessUniform {
    /// x=threshold, y=knee, z=scatter, w=intensity (0 when disabled)
    pub bloom: [f32; 4],
    /// Columns of the white balance matrix (linear RGB), w=unused
    pub white_balance: [[f32; 4]; 3],
    /// x=exposure multiplier, y=contrast, z=saturation, w=tonemapper
    pub grading: [f32; 4],
    pub color_filter: [f32; 4],
    pub lift: [f32; 4],
    pub gamma: [f32; 4],
    pub gain: [f32; 4],
    /// x=intensity, y=radius, z=smoothness, w=roundness
    pub vignette: [f32; 4],
    pub vignette_color: [f32; 4],
    /// x=chromatic aberration, y=grain intensity, z=grain response, w=dithering
    pub effects: [f32; 4],
    /// x=time in seconds
    pub time: [f32; 4],
    /// x=anti-aliasing method
    pub anti_aliasing: [f32; 4]
}
impl Default for PostProcessUniform {
    fn default() -> Self {
        Self::from_settings(&PostProcessSettings::default(), 0.0)
    }
}
impl PostProcessUniform {
    pub fn from_settings(settings: &PostProcessSettings, time: f32) -> Self {
        let bloom = &settings.bloom;
        let grading = &settings.color_grading;
        let vignette = &settings.vignette;
        let wb = white_balance(grading.temperature, grading.tint);
        let rgb = |c: [f32; 3]| [c[0], c[1], c[2], 0.0];
        Self {
            bloom: [
                bloom.threshold.max(0.0),
                bloom.threshold.max(0.0) * bloom.threshold_softness.clamp(0.0, 1.0),
                bloom.scatter.clamp(0.0, 1.0),
                if bloom.enabled {
                    bloom.intensity.max(0.0)
                } else {
                    0.0
                }
            ],
            white_balance: [wb.x_axis, wb.y_axis, wb.z_axis].map(|c| c.extend(0.0).to_array()),
            grading: [
                grading.exposure.exp2(),
                grading.contrast.max(0.01),
                grading.saturation.max(0.0),
                settings.tonemapping as u32 as f32
            ],
            color_filter: rgb(grading.color_filter),
            lift: rgb(grading.lift),
            gamma: rgb(grading.gamma.map(|g| g.max(0.01))),
            gain: rgb(grading.gain),
            vignette: [
                vignette.intensity.clamp(0.0, 1.0),
                vignette.radius,
                vignette.smoothness.max(0.001),
                vignette.roundness.clamp(0.0, 1.0)
            ],
            vignette_color: rgb(vignette.color),
            effects: [
                settings.chromatic_aberration.intensity.max(0.0) * 0.01,
                settings.film_grain.intensity.max(0.0),
                settings.film_grain.response.clamp(0.0, 1.0),
                if settings.dithering { 1.0 } else { 0.0 }
            ],
            time: [time, 0.0, 0.0, 0.0],
            anti_aliasing: [settings.anti_aliasing as u32 as f32, 0.0, 0.0, 0.0]
        }
    }
}

/// GPU layout of the fog parameters.
#[repr(C)]
#[derive(Resource, Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct FogUniform {
    /// xyz=color, w=sky color amount
    pub color: [f32; 4],
    /// x=density, y=height falloff, z=base height, w=start distance
    pub params: [f32; 4],
    /// x=max opacity, y=sky distance, z=enabled
    pub limits: [f32; 4],
    /// xyz=sun color, w=sun exponent
    pub sun: [f32; 4],
    /// x=sun intensity
    pub sun_params: [f32; 4]
}
impl Default for FogUniform {
    fn default() -> Self {
        Self::from_settings(&FogSettings::default())
    }
}
impl FogUniform {
    pub fn from_settings(fog: &FogSettings) -> Self {
        let [r, g, b] = fog.color;
        let [sr, sg, sb] = fog.sun_color;
        Self {
            color: [r, g, b, fog.sky_color.clamp(0.0, 1.0)],
            params: [
                fog.density.max(0.0),
                fog.height_falloff.max(0.0),
                fog.base_height,
                fog.start_distance.max(0.0)
            ],
            limits: [
                fog.max_opacity.clamp(0.0, 1.0),
                fog.sky_distance.max(0.0),
                if fog.enabled { 1.0 } else { 0.0 },
                0.0
            ],
            sun: [sr, sg, sb, fog.sun_exponent.max(1.0)],
            sun_params: [fog.sun_intensity.max(0.0), 0.0, 0.0, 0.0]
        }
    }
}

// Linear RGB <-> LMS cone response
const LIN_TO_LMS: Mat3 = Mat3::from_cols(
    Vec3::new(3.904_05e-1, 7.084_16e-2, 2.310_82e-2),
    Vec3::new(5.499_41e-1, 9.631_72e-1, 1.280_21e-1),
    Vec3::new(8.926_32e-3, 1.357_75e-3, 9.362_45e-1)
);
const LMS_TO_LIN: Mat3 = Mat3::from_cols(
    Vec3::new(2.858_47, -2.101_82e-1, -4.181_20e-2),
    Vec3::new(-1.628_79, 1.158_20, -1.181_69e-1),
    Vec3::new(-2.489_10e-2, 3.242_81e-4, 1.068_67)
);

/// White balance matrix, as a von Kries adaptation from a shifted D65 illuminant.
fn white_balance(temperature: f32, tint: f32) -> Mat3 {
    let t1 = temperature.clamp(-1.0, 1.0) * 100.0 / 65.0;
    let t2 = tint.clamp(-1.0, 1.0) * 100.0 / 65.0;
    let x = 0.31271 - t1 * if t1 < 0.0 { 0.1 } else { 0.05 };
    let y = 2.87 * x - 3.0 * x * x - 0.275_095_07 + t2 * 0.05;

    // CIE xy to LMS, with Y = 1
    let xyz = Vec3::new(x / y, 1.0, (1.0 - x - y) / y);
    let lms = Vec3::new(
        0.7328 * xyz.x + 0.4296 * xyz.y - 0.1624 * xyz.z,
        -0.7036 * xyz.x + 1.6975 * xyz.y + 0.0061 * xyz.z,
        0.0030 * xyz.x + 0.0136 * xyz.y + 0.9834 * xyz.z
    );
    let d65 = Vec3::new(0.949_237, 1.035_42, 1.087_28);
    LMS_TO_LIN * Mat3::from_diagonal(d65 / lms) * LIN_TO_LMS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_white_balance_is_identity() {
        let wb = white_balance(0.0, 0.0);
        let diff = wb - Mat3::IDENTITY;
        for c in [diff.x_axis, diff.y_axis, diff.z_axis] {
            assert!(c.abs().max_element() < 0.01, "{wb:?}");
        }
    }

    #[test]
    fn warm_white_balance_boosts_red() {
        let white = white_balance(0.5, 0.0) * Vec3::ONE;
        assert!(white.x > white.z);
    }
}
