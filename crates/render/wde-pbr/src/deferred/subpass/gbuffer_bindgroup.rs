use bevy::prelude::*;

use bevy::ecs::system::SystemParamItem;
use wde_renderer::prelude::*;

use crate::{deferred::transform::PbrSsboTransform, prelude::PbrSsboInstanceToTransform};

#[derive(Default, Clone, TypePath, Asset)]
pub struct SsboTransformBinding;
impl RenderBinding for SsboTransformBinding {
    type Params = (
        SRenderData<PbrSsboTransform>,
        SRenderData<PbrSsboInstanceToTransform>
    );

    fn describe(
        &mut self,
        (ssbo_transform, ssbo_instance_to_transform): &SystemParamItem<Self::Params>,
        builder: &mut wde_renderer::assets::RenderBindingBuilder
    ) {
        builder
            .add_buffer(ssbo_transform, PbrSsboTransform::TRANSFORM_IDX)
            .add_buffer(
                ssbo_instance_to_transform,
                PbrSsboInstanceToTransform::INSTANCE_TO_TRANSFORM_IDX
            );
    }

    fn label(&self) -> &str {
        "ssbo-transform-bindgroup"
    }
}
