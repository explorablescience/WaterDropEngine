use wde_renderer::prelude::*;

use bevy::prelude::*;

use crate::logic::{batches::BatchesPlugin, lights::LightsFeature, ssbo::PbrSsboPlugin, textures::{PbrDeferredTextures, PbrDeferredTexturesLayout}};

pub mod ssbo;
pub mod textures;
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

