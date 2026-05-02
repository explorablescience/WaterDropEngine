use bevy::ecs::system::SystemParamItem;
use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::passes::shadow::ShadowMapData;

/// Bind group for the shadow map pass vertex shader (group 1): light view-proj uniform.
#[derive(Default, Clone, TypePath, Asset)]
pub struct ShadowParamsBinding;
impl RenderBinding for ShadowParamsBinding {
    type Params = SRenderData<ShadowMapData>;

    fn describe(
        &mut self,
        shadow_data: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder.add_buffer(shadow_data, ShadowMapData::PARAMS_BUFFER_IDX);
    }

    fn has_dependencies(&self) -> bool {
        true
    }

    fn label(&self) -> &str {
        "shadow-params-binding"
    }
}

/// Bind group for the deferred lighting shader (group 3): shadow map texture + sampler + params.
#[derive(Default, Clone, TypePath, Asset)]
pub struct ShadowSamplingBinding;
impl RenderBinding for ShadowSamplingBinding {
    type Params = SRenderData<ShadowMapData>;

    fn describe(
        &mut self,
        shadow_data: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder
            .add_depth_texture_view(shadow_data, ShadowMapData::SHADOW_MAP_IDX)
            .add_texture_sampler(shadow_data, ShadowMapData::SHADOW_MAP_IDX)
            .add_buffer(shadow_data, ShadowMapData::PARAMS_BUFFER_IDX);
    }

    fn has_dependencies(&self) -> bool {
        true
    }

    fn label(&self) -> &str {
        "shadow-sampling-binding"
    }
}
