use bevy::{
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    prelude::*
};
use wde_renderer::prelude::{Color, *};

#[repr(C)]
#[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SelectionAreaMaterialUniform {
    color: [f32; 4]
}

/// A flat, alpha-blended material used to draw the RTS-style ground selection decal
/// (the drag-rectangle highlight) on top of the terrain.
#[derive(Asset, TypePath, Clone)]
pub struct SelectionAreaMaterial {
    pub label: String,
    pub color: Color,
    pub uniform_buffer: Option<Handle<Buffer>>
}
impl Default for SelectionAreaMaterial {
    fn default() -> Self {
        Self {
            label: "selection-area-material".to_string(),
            color: Color::LinearRgba(1.0, 0.85, 0.0, 0.25),
            uniform_buffer: None
        }
    }
}
impl RenderBinding for SelectionAreaMaterial {
    type Params = SRes<AssetServer>;

    fn describe(
        &mut self,
        asset_server: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        let uniform = SelectionAreaMaterialUniform {
            color: [
                self.color.r(),
                self.color.g(),
                self.color.b(),
                self.color.a()
            ]
        };

        if self.uniform_buffer.is_none() {
            self.uniform_buffer = Some(asset_server.add(Buffer {
                label: format!("{}-uniform", self.label),
                size: std::mem::size_of::<SelectionAreaMaterialUniform>(),
                usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                content: Some(bytemuck::cast_slice(&[uniform]).to_vec())
            }));
        }

        builder.add_buffer_from_id(Some(self.uniform_buffer.as_ref().unwrap().id()));
    }

    fn label(&self) -> &str {
        &self.label
    }
}
