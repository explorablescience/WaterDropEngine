use bevy::prelude::*;
use pass_resolve::*;
use wde_renderer::prelude::*;

mod atmosphere;
mod pass_deferred_gbuffer;
mod pass_deferred_lighting;
mod pass_resolve;
mod pass_transparent;
pub mod shadow;

pub use atmosphere::*;
pub use pass_deferred_gbuffer::*;
pub use pass_deferred_lighting::*;
pub use pass_transparent::*;
pub use shadow::*;

pub(crate) struct PassesPlugin;
impl Plugin for PassesPlugin {
    fn build(&self, app: &mut App) {
        // Register shadow settings in the main world
        app.init_resource::<CascadedShadowSettings>();

        // Add the render graph nodes
        let render_world = app.get_sub_app_mut(RenderApp).unwrap();
        render_world
            .init_resource::<ExtractedCascadedShadowParams>()
            .init_resource::<CascadeEnabledFlags>()
            .add_systems(Extract, shadow::extract_cascaded_shadow_params)
            .add_systems(Extract, shadow::extract_cascade_enabled_flags)
            .add_systems(
                Render,
                shadow::update_cascaded_shadow_params_buffer.in_set(RenderSet::Prepare)
            );

        let mut render_graph = render_world
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap();
        render_graph
            .add_pass::<RenderPassShadow>()
            .add_sub_pass::<SubRenderPassShadow, RenderPassShadow>()
            .add_pass::<RenderPassShadow1>()
            .add_sub_pass::<SubRenderPassShadow1, RenderPassShadow1>()
            .add_pass::<RenderPassShadow2>()
            .add_sub_pass::<SubRenderPassShadow2, RenderPassShadow2>()
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

        // Add the ui
        #[cfg(debug_assertions)]
        app.add_systems(Update, shadow::params_data_ui);

        // Add the pipelines and bindings
        app.add_plugins((
            RenderDataRegisterPlugin::<ShadowMapData>::default(),
            RenderBindingRegisterPlugin::<ShadowParamsBinding>::default(),
            RenderBindingRegisterPlugin::<ShadowSamplingBinding>::default(),
            RenderPipelineRegisterPlugin::<ShadowMapPipeline>::default(),
            RenderPipelineRegisterPlugin::<ShadowMapPipeline1>::default(),
            RenderPipelineRegisterPlugin::<ShadowMapPipeline2>::default(),
            RenderBindingRegisterPlugin::<RenderBindingResolved>::default(),
            RenderBindingRegisterPlugin::<LightsDataBinding>::default(),
            #[cfg(feature = "atmosphere")]
            RenderBindingRegisterPlugin::<AtmosphereDepthBinding>::default(),
            RenderPipelineRegisterPlugin::<ResolveRenderPipeline>::default(),
            RenderPipelineRegisterPlugin::<DeferredLightingPipeline>::default(),
            #[cfg(feature = "atmosphere")]
            RenderPipelineRegisterPlugin::<AtmospherePipeline>::default(),
            #[cfg(all(feature = "atmosphere", debug_assertions))]
            atmosphere::editor::AtmosphereEditorPlugin,
            #[cfg(feature = "atmosphere")]
            AtmosphereSunLightPlugin,
            ExtractResourcePlugin::<CascadedShadowSettings>::default()
        ));
    }
}
