use bevy::prelude::*;
use pass_resolve::*;
use wde_renderer::prelude::*;

mod pass_deferred_gbuffer;
mod pass_deferred_lighting;
mod pass_resolve;
mod pass_transparent;

pub use pass_deferred_gbuffer::*;
pub use pass_deferred_lighting::*;
pub use pass_transparent::*;

pub(crate) struct PassesPlugin;
impl Plugin for PassesPlugin {
    fn build(&self, app: &mut App) {
        // Add the render graph nodes
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap()
            .add_pass::<RenderPassDeferredGBuffer>()
            .add_pass::<RenderPassDeferredLighting>()
            .add_pass::<RenderPassTransparent>()
            .add_pass::<RenderPassResolve>()
            .add_sub_pass::<SubRenderPassResolve, RenderPassResolve>();

        // Add the pipelines and bindings
        app.add_plugins((
            RenderBindingRegisterPlugin::<RenderBindingResolved>::default(),
            RenderPipelineRegisterPlugin::<ResolveRenderPipeline>::default(),
            RenderPipelineRegisterPlugin::<DeferredLightingPipeline>::default()
        ));
    }
}
