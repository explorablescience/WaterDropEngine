use bevy::prelude::*;
use depth_texture_msaa::{DepthTextureMSAA};
use depth_texture::{DepthTexture};

use crate::{core::{Extract, Render, RenderApp, RenderSet}, passes::depth::depth_texture_msaa::DepthMSAATextureBindGroup};

pub mod depth_texture;
pub mod depth_texture_msaa;

pub(crate) struct RendererPlugin;
impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        // Add the depth MSAA texture to the app
        app
            .add_systems(Startup, DepthTextureMSAA::init)
            .add_systems(Update, DepthTextureMSAA::resize);
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Extract, DepthTextureMSAA::extract)
            .init_resource::<DepthMSAATextureBindGroup>()
            .add_systems(Render, DepthMSAATextureBindGroup::build_bind_group.in_set(RenderSet::BindGroups));

        // Add the depth texture to the app
        app
            .add_systems(Startup, DepthTexture::create_texture)
            .add_systems(Update, DepthTexture::resize_texture);
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Extract, DepthTexture::extract_texture);
    }
}
