use bevy::prelude::*;

/// List of all types of brushes that can be used for terrain editing
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Reflect)]
#[reflect(PartialEq, Hash, Debug)]
pub enum PaintMode {
    // Painting modes (edit splatmaps)
    #[default]
    Paint,
    Erase,

    // Non-painting modes (edit heightmap)
    Raise,
    Lower,
    Smooth,
    Flatten
}

/// The generic paint command generated from a brush.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PaintCommand {
    pub world_position: Vec3,
    pub radius: f32,
    pub strength: f32,
    pub color: [f32; 4],
    pub paint_mode: PaintMode
}

/// A general brush used to paint
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct PaintBrush {
    pub radius: f32,
    pub strength: f32,
    pub color: [f32; 4],
    pub paint_mode: PaintMode
}
impl Default for PaintBrush {
    fn default() -> Self {
        Self {
            radius: 10.0,
            strength: 1.0,
            color: [1.0, 0.0, 0.0, 1.0],
            paint_mode: PaintMode::Paint
        }
    }
}
impl PaintBrush {
    pub fn paint(&self, world_position: Vec3) -> PaintCommand {
        PaintCommand {
            world_position,
            radius: self.radius,
            strength: self.strength,
            color: self.color,
            paint_mode: self.paint_mode
        }
    }
}
