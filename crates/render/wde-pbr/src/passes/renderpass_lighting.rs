use bevy::ecs::system::SystemParamItem;
use wde_renderer::prelude::*;


pub struct PbrLightingRenderPass;
impl RenderPass for PbrLightingRenderPass {
    type Params = ();

    fn describe(_params: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc::default()
    }

    fn id() -> RenderPassId { 51 }
    fn label() -> &'static str { "pbr-lighting" }
}
