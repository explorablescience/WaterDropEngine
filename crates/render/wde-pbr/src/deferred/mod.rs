use bevy::prelude::*;
use wde_renderer::prelude::*;

mod dependencies;
mod lights;
mod subpass;

pub use dependencies::*;
pub use lights::*;

use crate::{
    deferred::{
        dependencies::DeferredDependenciesPlugin, lights::LightsPlugin, subpass::PbrRenderPlugin
    },
    passes::{RenderPassDeferredLighting, SubRenderPassLightingPbr}
};

pub(crate) struct DeferredPlugin;
impl Plugin for DeferredPlugin {
    fn build(&self, app: &mut App) {
        // Add the plugins
        app.add_plugins((LightsPlugin, DeferredDependenciesPlugin, PbrRenderPlugin));

        // Add the render graph nodes
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap()
            .add_sub_pass::<SubRenderPassLightingPbr, RenderPassDeferredLighting>();
    }
}
