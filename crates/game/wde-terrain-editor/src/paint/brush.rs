use bevy::prelude::*;

/// List of all types of brushes that can be used for terrain editing
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum BrushType {
    #[default]
    Paint
}

/// The generic paint command generated from a brush.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PaintCommand {
    pub world_position: Vec3,
    pub radius: f32,
    pub strength: f32,
    pub color: [f32; 3],
    pub brush_type: BrushType
}

/// A general brush used to paint
#[derive(Component, Default)]
pub struct PaintingBrush {
    pub radius: f32,
    pub strength: f32,
    pub color: [f32; 3],
    pub brush_type: BrushType
}
impl PaintingBrush {
    pub fn paint(&self, world_position: Vec3) -> PaintCommand {
        PaintCommand {
            world_position,
            radius: self.radius,
            strength: self.strength,
            color: self.color,
            brush_type: self.brush_type
        }
    }
}

