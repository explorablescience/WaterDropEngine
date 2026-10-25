use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_renderer::prelude::*;

use crate::render::{data::UiQuadsBinding, image::UiImageLayout};

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct UiRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for UiRenderPipeline {
    type SourceAsset = RenderPipelineAsset<UiRenderPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<UiQuadsBinding>,
        SRes<UiImageLayout>
    );

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, ui_quads, image_layout): &mut SystemParamItem<
            Self::Params
        >
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(UiRenderPipeline(pipeline_manager.create_render_pipeline(
            RenderPipelineDescriptor {
                label: "ui-render",
                vert: Some(assets_server.load("core/render/ui/vert.wgsl")),
                frag: Some(assets_server.load("core/render/ui/frag.wgsl")),
                bind_group_layouts: vec![
                    ui_quads.iter().next().map(|(_, b)| b.layout.clone()),
                    Some(image_layout.0.clone()),
                ],
                fragment_blend: Some(BlendState::ALPHA_BLENDING),
                cull_mode: None,
                // Quads are pulled from the storage buffer by instance index
                vertex_buffer: false,
                ..default()
            },
            asset
        )?))
    }
}
