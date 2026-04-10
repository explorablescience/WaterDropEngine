use bevy::prelude::*;

use crate::{
    assets::bindings::{builder::RenderBindingsBuilderCache, placeholder_texture::DummyTexturePlugin},
    core::RenderApp
};

mod builder;
mod placeholder_texture;
mod material;
mod render_binding;
mod render_binding_old;
mod render_data;

pub use builder::RenderBindingBuilderOld;
pub use material::*;
pub use render_binding::*;
pub use render_binding_old::*;
pub use render_data::{
    GpuRenderData, RenderData, RenderDataBuilder, RenderDataRegisterPlugin, ResMutRenderData,
    ResRenderData, SRenderData, SRenderDataMut
};
pub use placeholder_texture::PlaceholderTexture;

pub(crate) struct MaterialsPlugin;
impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        // Add the dummy texture plugin to have a default white texture
        app.add_plugins(DummyTexturePlugin);

        // Add cached resources
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<RenderBindingsBuilderCache>();
    }
}
