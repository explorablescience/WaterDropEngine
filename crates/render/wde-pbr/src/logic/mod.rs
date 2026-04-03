use wde_renderer::prelude::*;

use bevy::prelude::*;

use crate::logic::{batches::BatchesPlugin, deferred_textures::{PbrDeferredTextures, PbrDeferredTexturesLayout}, lights::LightsFeature, render_texture::{PbrRenderTexture, PbrRenderTextureBindGroup}, ssbo::PbrSsboPlugin};

pub mod ssbo;
pub mod render_texture;
pub mod deferred_textures;
pub mod batches;
pub mod lights;

pub(crate) struct PbrLogicPlugin;
impl Plugin for PbrLogicPlugin {
    fn build(&self, app: &mut App) {
        // Add the pbr ssbo
        app
            .add_plugins(LightsFeature)
            .add_plugins(PbrSsboPlugin)
            .add_plugins(BatchesPlugin);

        // Add the pbr render texture
        app
            .add_systems(Startup, PbrRenderTexture::create_texture)
            .add_systems(Update, PbrRenderTexture::resize_texture);
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<PbrRenderTextureBindGroup>()
            .add_systems(Extract, PbrRenderTexture::extract_texture)
            .add_systems(Render, PbrRenderTextureBindGroup::build_bind_group.in_set(RenderSet::BindGroups));

        // Add the pbr defered textures
        app
            .add_systems(Startup, PbrDeferredTextures::create_textures)
            .add_systems(Update, PbrDeferredTextures::resize_textures);
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<PbrDeferredTexturesLayout>()
            .add_systems(Extract, PbrDeferredTextures::extract_textures)
            .add_systems(Render, PbrDeferredTexturesLayout::build_bind_group.in_set(RenderSet::BindGroups));
    }
}

