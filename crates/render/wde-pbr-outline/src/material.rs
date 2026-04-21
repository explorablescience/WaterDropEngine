use bevy::{
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    prelude::*
};
use wde_renderer::prelude::{Color, *};

#[repr(C)]
#[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct OutlineMaterialUniform {
    color: [f32; 4],
    thickness: f32,
    _padding: [f32; 3]
}

#[derive(Asset, TypePath, Clone)]
pub struct OutlineMaterial {
    pub label: String,
    pub color: Color,
    pub thickness: f32,
    pub uniform_buffer: Option<Handle<Buffer>>
}
impl Default for OutlineMaterial {
    fn default() -> Self {
        Self {
            label: "outline-material".to_string(),
            color: Color::LinearRgba(1.0, 1.0, 1.0, 0.6),
            thickness: 0.02,
            uniform_buffer: None
        }
    }
}
impl RenderBinding for OutlineMaterial {
    type Params = SRes<AssetServer>;

    fn describe(
        &mut self,
        asset_server: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        let uniform = OutlineMaterialUniform {
            color: [
                self.color.r(),
                self.color.g(),
                self.color.b(),
                self.color.a()
            ],
            thickness: self.thickness,
            _padding: [0.0, 0.0, 0.0]
        };

        if self.uniform_buffer.is_none() {
            self.uniform_buffer = Some(asset_server.add(Buffer {
                label: format!("{}-uniform", self.label),
                size: std::mem::size_of::<OutlineMaterialUniform>(),
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
