use bevy::prelude::*;
use pass_resolve::*;
use wde_renderer::prelude::*;

#[cfg(feature = "atmosphere")]
mod atmosphere;
mod pass_deferred_gbuffer;
mod pass_deferred_lighting;
mod pass_resolve;
mod pass_transparent;

#[cfg(feature = "atmosphere")]
pub use atmosphere::*;
pub use pass_deferred_gbuffer::*;
pub use pass_deferred_lighting::*;
pub use pass_transparent::*;

pub(crate) struct PassesPlugin;
impl Plugin for PassesPlugin {
    fn build(&self, app: &mut App) {
        // Add the render graph nodes
        let mut render_graph = app
            .get_sub_app_mut(RenderApp)
            .unwrap()
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap();
        render_graph
            .add_pass::<RenderPassDeferredGBuffer>()
            .add_pass::<RenderPassDeferredLighting>();
        #[cfg(feature = "atmosphere")]
        render_graph.add_pass::<RenderPassAtmosphere>();
        render_graph
            .add_pass::<RenderPassTransparent>()
            .add_pass::<RenderPassResolve>()
            .add_sub_pass::<SubRenderPassResolve, RenderPassResolve>();
        #[cfg(feature = "atmosphere")]
        render_graph.add_sub_pass::<SubRenderPassAtmosphere, RenderPassAtmosphere>();

        // Add the pipelines and bindings
        app.add_plugins((
            RenderBindingRegisterPlugin::<RenderBindingResolved>::default(),
            RenderBindingRegisterPlugin::<LightsDataBinding>::default(),
            #[cfg(feature = "atmosphere")]
            RenderBindingRegisterPlugin::<AtmosphereDepthBinding>::default(),
            RenderPipelineRegisterPlugin::<ResolveRenderPipeline>::default(),
            RenderPipelineRegisterPlugin::<DeferredLightingPipeline>::default(),
            #[cfg(feature = "atmosphere")]
            RenderPipelineRegisterPlugin::<AtmospherePipeline>::default(),
            #[cfg(debug_assertions)]
            atmosphere::editor::AtmosphereEditorPlugin,
        ));
    }
}
