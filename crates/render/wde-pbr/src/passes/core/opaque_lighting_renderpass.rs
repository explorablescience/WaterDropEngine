use bevy::ecs::system::SystemParamItem;
use wde_renderer::prelude::*;


pub struct RenderPassOpaqueLighting;
impl RenderPass for RenderPassOpaqueLighting {
    type Params = ();

    fn describe(_params: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc::default()
    }

    fn id() -> RenderPassId { 20 }
    fn label() -> &'static str { "pbr-lighting" }
}
