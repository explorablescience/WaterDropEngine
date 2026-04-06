use bevy::prelude::*;
use wde_renderer::prelude::*;

mod lights;
mod dependencies;
mod subpass;

pub use lights::*;
pub use dependencies::*;
pub use subpass::*;

use crate::{deferred::{lights::LightsPlugin, dependencies::PbrDependenciesPlugin, subpass::PbrRenderPlugin}, passes::{RenderPassDeferredLighting, SubRenderPassLightingPbr}};

pub(crate) struct DeferredPlugin;
impl Plugin for DeferredPlugin {
    fn build(&self, app: &mut App) {
        // Add the plugins
        app.add_plugins((
            LightsPlugin,
            PbrDependenciesPlugin,
            PbrRenderPlugin
        ));

        // Add the render graph nodes
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap()
            .add_sub_pass::<SubRenderPassLightingPbr, RenderPassDeferredLighting>();
    }
}
