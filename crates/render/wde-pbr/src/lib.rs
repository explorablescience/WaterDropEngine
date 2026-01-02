#![allow(clippy::type_complexity)]
use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::{assets::PbrAssetsPlugin, components::PbrComponentsPlugin, features::PbrFeaturesPlugin, passes::{PbrFeaturesPlugin as PbrPassesPlugin, PbrGBufferRenderPass, PbrLightingRenderPass}};

pub mod prelude {
    pub use crate::PbrPlugin;
    pub use crate::assets::{PbrMaterial, PbrMaterialAsset};
    pub use crate::components::lights::*;
}

pub mod assets;
pub mod components;
pub mod features;
pub mod passes;

pub struct PbrPlugin;
impl Plugin for PbrPlugin {
    fn build(&self, app: &mut App) {
        // Add the different plugins
        app
            .add_plugins(PbrAssetsPlugin)
            .add_plugins(PbrComponentsPlugin)
            .add_plugins(PbrPassesPlugin)
            .add_plugins(PbrFeaturesPlugin);

        // Add the pbr render passes
        let mut render_graph = app.get_sub_app_mut(RenderApp).unwrap()
            .world_mut().get_resource_mut::<RenderGraph>().unwrap();
        render_graph.add_pass::<PbrGBufferRenderPass>(0);
        render_graph.add_pass::<PbrLightingRenderPass>(1);
    }
}
