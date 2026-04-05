use bevy::{
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    prelude::*
};
use wde_renderer::prelude::*;
use wde_camera::prelude::*;

use crate::{
    logic::{deferred_textures::PbrDeferredTexturesLayout, lights::LightsFeatureBuffer},
    passes::subpass::lighting_pipeline::GpuPbrLightingRenderPipeline
};

pub struct SubRenderPassLightingPbr;
impl RenderSubPass for SubRenderPassLightingPbr {
    type Params = (
        SRes<RenderAssets<GpuPbrLightingRenderPipeline>>,
        SRes<PostProcessingMesh>,
        SBinding<CameraRender>,
        SRes<PbrDeferredTexturesLayout>,
        SRes<LightsFeatureBuffer>
    );

    fn describe(
        (pipeline, mesh, camera, deferred_textures_layout, lights_buffer): &SystemParamItem<
            Self::Params
        >
    ) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(Some(pipeline.iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::Mesh(mesh.0.as_ref().map(|m| m.id())),
            SubPassCommand::BindGroup(0, camera.iter().next().map(|(_, camera)| camera.bind_group.clone())),
            SubPassCommand::BindGroup(
                1,
                deferred_textures_layout
                    .deferred_bind_group_resolved
                    .clone()
            ),
            SubPassCommand::BindGroup(2, lights_buffer.bind_group.clone()),
            SubPassCommand::DrawBatches(vec![DrawCommandsBatch {
                index_range: 0..6,
                ..Default::default()
            }]),
        ])
    }

    fn label() -> &'static str {
        "pbr-lighting-main"
    }
}
