use bevy::prelude::*;
use depth::{DepthTexture, DepthTextureLayout};
use gizmo::GizmoFeaturesPlugin;
use pbr::PbrFeaturesPlugin;

use crate::core::{Extract, Render, RenderApp, RenderSet};

pub mod pbr;
pub mod depth;
pub mod gizmo;
pub mod render_graph;

pub(crate) struct RendererPlugin;
impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        // Add the depth texture to the app
        app
            .add_systems(Startup, DepthTexture::create_texture)
            .add_systems(Update, DepthTexture::resize_texture);
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<DepthTextureLayout>()
            .add_systems(Extract, DepthTexture::extract_texture)
            .add_systems(Render, DepthTextureLayout::build_bind_group.in_set(RenderSet::BindGroups));

        // Add the different render passes to the app
        app
            .add_plugins(PbrFeaturesPlugin)
            .add_plugins(GizmoFeaturesPlugin);
    }
}
