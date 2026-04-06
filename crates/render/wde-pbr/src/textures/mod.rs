mod depth;
mod render_texture;
mod deferred_textures;

pub use render_texture::*;
pub use depth::*;
pub use deferred_textures::*;

use wde_renderer::prelude::*;

use bevy::prelude::*;

use crate::textures::{depth::DepthTexturePlugin};

pub(crate) struct CorePlugin;
impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        // Add the msaa render texture
        app.add_systems(Startup, RenderTexture::create_texture)
            .add_systems(Update, RenderTexture::resize_texture);
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<RenderTextureBindGroup>()
            .add_systems(Extract, RenderTexture::extract_texture)
            .add_systems(
                Render,
                RenderTextureBindGroup::build_bind_group.in_set(RenderSet::BindGroups)
            );

        // Add the depth texture
        app.add_plugins(DepthTexturePlugin);

        // Add the defered textures
        app.add_systems(Startup, PbrDeferredTextures::create_textures)
            .add_systems(Update, PbrDeferredTextures::resize_textures);
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<PbrDeferredTexturesLayout>()
            .add_systems(Extract, PbrDeferredTextures::extract_textures)
            .add_systems(
                Render,
                PbrDeferredTexturesLayout::build_bind_group.in_set(RenderSet::BindGroups),
            );
    }
}
