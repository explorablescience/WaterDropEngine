//! Lightweight color helpers used by render assets and materials.
//!
//! The enum wraps either linear or sRGB encoded values and provides cheap conversions so render code can stay explicit about color space. All values are expected to be in the 0.0–1.0 range and are left un-clamped so callers can decide how to handle HDR or out-of-gamut data.

use bevy::reflect::Reflect;
use wde_wgpu::passes::WgpuColor;

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub enum WdeColor {
    /// Linear RGBA color expressed in normalized `[0.0, 1.0]` space.
    LinearRgba(f32, f32, f32, f32),
    /// sRGB RGBA color expressed in normalized `[0.0, 1.0]` space.
    Srgba(f32, f32, f32, f32)
}
impl WdeColor {
    // Default colors
    pub const BLACK: WdeColor = WdeColor::LinearRgba(0.0, 0.0, 0.0, 1.0);
    pub const WHITE: WdeColor = WdeColor::LinearRgba(1.0, 1.0, 1.0, 1.0);
    pub const RED: WdeColor = WdeColor::LinearRgba(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: WdeColor = WdeColor::LinearRgba(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: WdeColor = WdeColor::LinearRgba(0.0, 0.0, 1.0, 1.0);
    pub const TRANSPARENT: WdeColor = WdeColor::LinearRgba(0.0, 0.0, 0.0, 0.0);

    /// Create a linear RGBA color.
    pub fn from_linear_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        WdeColor::LinearRgba(r, g, b, a)
    }
    /// Create an sRGB RGBA color.
    pub fn from_srgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        WdeColor::Srgba(r, g, b, a)
    }

    /// Convert the color to linear RGBA (gamma ≈ 2.2).
    pub fn to_linear_rgba(&self) -> Self {
        match self {
            WdeColor::LinearRgba(_, _, _, _) => *self,
            WdeColor::Srgba(r, g, b, a) => {
                let r = r.powf(2.2);
                let g = g.powf(2.2);
                let b = b.powf(2.2);
                WdeColor::LinearRgba(r, g, b, *a)
            }
        }
    }
    /// Convert the color to sRGB RGBA (inverse gamma ≈ 2.2).
    pub fn to_srgba(&self) -> Self {
        match self {
            WdeColor::Srgba(_, _, _, _) => *self,
            WdeColor::LinearRgba(r, g, b, a) => {
                let r = r.powf(1.0 / 2.2);
                let g = g.powf(1.0 / 2.2);
                let b = b.powf(1.0 / 2.2);
                WdeColor::Srgba(r, g, b, *a)
            }
        }
    }

    /// Get the red component without converting color space.
    pub fn r(&self) -> f32 {
        match self {
            WdeColor::LinearRgba(r, _, _, _) => *r,
            WdeColor::Srgba(r, _, _, _) => *r
        }
    }
    /// Get the green component without converting color space.
    pub fn g(&self) -> f32 {
        match self {
            WdeColor::LinearRgba(_, g, _, _) => *g,
            WdeColor::Srgba(_, g, _, _) => *g
        }
    }
    /// Get the blue component without converting color space.
    pub fn b(&self) -> f32 {
        match self {
            WdeColor::LinearRgba(_, _, b, _) => *b,
            WdeColor::Srgba(_, _, b, _) => *b
        }
    }
    /// Get the alpha component without converting color space.
    pub fn a(&self) -> f32 {
        match self {
            WdeColor::LinearRgba(_, _, _, a) => *a,
            WdeColor::Srgba(_, _, _, a) => *a
        }
    }
}
impl From<WdeColor> for WgpuColor {
    fn from(value: WdeColor) -> Self {
        match value {
            WdeColor::LinearRgba(r, g, b, a) => WgpuColor {
                r: r.into(),
                g: g.into(),
                b: b.into(),
                a: a.into()
            },
            WdeColor::Srgba(r, g, b, a) => {
                let r = r.powf(2.2);
                let g = g.powf(2.2);
                let b = b.powf(2.2);
                WgpuColor {
                    r: r.into(),
                    g: g.into(),
                    b: b.into(),
                    a: a.into()
                }
            }
        }
    }
}
