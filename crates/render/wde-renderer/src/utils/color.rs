use bevy::reflect::Reflect;

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub enum Color {
    /// Linear RGBA color.
    LinearRgba(f32, f32, f32, f32),
    /// SRGB RGBA color.
    Srgba(f32, f32, f32, f32),
}
impl Color {
    /// Create a new linear RGBA color.
    pub fn from_linear_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Color::LinearRgba(r, g, b, a)
    }
    /// Create a new sRGB RGBA color.
    pub fn from_srgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Color::Srgba(r, g, b, a)
    }


    /// Convert the color to linear RGBA.
    pub fn to_linear_rgba(&self) -> Self {
        match self {
            Color::LinearRgba(_, _, _, _) => *self,
            Color::Srgba(r, g, b, a) => {
                let r = r.powf(2.2);
                let g = g.powf(2.2);
                let b = b.powf(2.2);
                Color::LinearRgba(r, g, b, *a)
            }
        }
    }
    /// Convert the color to sRGB RGBA.
    pub fn to_srgba(&self) -> Self {
        match self {
            Color::Srgba(_, _, _, _) => *self,
            Color::LinearRgba(r, g, b, a) => {
                let r = r.powf(1.0 / 2.2);
                let g = g.powf(1.0 / 2.2);
                let b = b.powf(1.0 / 2.2);
                Color::Srgba(r, g, b, *a)
            }
        }
    }


    /// Get the red component.
    pub fn r(&self) -> f32 {
        match self {
            Color::LinearRgba(r, _, _, _) => *r,
            Color::Srgba(r, _, _, _) => *r,
        }
    }
    /// Get the green component.
    pub fn g(&self) -> f32 {
        match self {
            Color::LinearRgba(_, g, _, _) => *g,
            Color::Srgba(_, g, _, _) => *g,
        }
    }
    /// Get the blue component.
    pub fn b(&self) -> f32 {
        match self {
            Color::LinearRgba(_, _, b, _) => *b,
            Color::Srgba(_, _, b, _) => *b,
        }
    }
    /// Get the alpha component.
    pub fn a(&self) -> f32 {
        match self {
            Color::LinearRgba(_, _, _, a) => *a,
            Color::Srgba(_, _, _, a) => *a,
        }
    }
}
