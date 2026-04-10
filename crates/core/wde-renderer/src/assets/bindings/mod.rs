use bevy::prelude::*;

use crate::assets::bindings::placeholder_texture::DummyTexturePlugin;

mod placeholder_texture;
mod render_binding;
mod render_data;

pub use placeholder_texture::PlaceholderTexture;
pub use render_binding::*;
pub use render_data::{
    GpuRenderData, RenderData, RenderDataBuilder, RenderDataRegisterPlugin, ResMutRenderData,
    ResRenderData, SRenderData, SRenderDataMut
};

pub(crate) struct MaterialsPlugin;
impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DummyTexturePlugin);
    }
}
