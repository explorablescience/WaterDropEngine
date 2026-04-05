use bevy::prelude::*;
use wde_renderer::prelude::*;

/// Describes a simple gizmo material with a color.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct GizmoMaterial(pub Handle<GizmoMaterialAsset>);

#[repr(C)]
#[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct GizmoMaterialUniform {
    /// Color of the material.
    pub color: [f32; 4],
}

#[derive(Asset, Clone, TypePath)]
/// Describes a simple gizmo material with a color.
pub struct GizmoMaterialAsset {
    /// The label of the material instance.
    pub label: String,
    /// The color of the material instance.
    pub color: [f32; 4],
}
impl Default for GizmoMaterialAsset {
    fn default() -> Self {
        GizmoMaterialAsset {
            label: "gizmo-material".to_string(),
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}
impl Material for GizmoMaterialAsset {}
impl RenderBinding for GizmoMaterialAsset {
    fn describe(&self, builder: &mut RenderBindingBuilder) {
        // Create the uniform buffer
        let uniform = GizmoMaterialUniform {
            color: self.color,
        };

        // Build the material
        builder.add_buffer(0, Buffer {
            label: format!("{}-uniform-buffer", self.label),
            size: std::mem::size_of::<GizmoMaterialUniform>(),
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
            content: Some(bytemuck::cast_slice(&[uniform]).to_vec())
        });
    }

    fn label(&self) -> &str {
        &self.label
    }
}
