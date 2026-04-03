use bevy::{ecs::system::{SystemParamItem, lifetimeless::SRes}, prelude::*};
use wde_camera::render::CameraFeatureRender;
use wde_renderer::prelude::*;

use crate::{logic::{lights::LightsFeatureBuffer, textures::PbrDeferredTexturesLayout}, passes::{PbrLightingRenderPassMesh, pipeline_lighting::GpuPbrLightingRenderPipeline}};


pub(crate) struct SubRenderPassLightingPbr;
impl RenderSubPass for SubRenderPassLightingPbr {
    type Params = (SRes<RenderAssets<GpuPbrLightingRenderPipeline>>, SRes<PbrLightingRenderPassMesh>, SRes<CameraFeatureRender>, SRes<PbrDeferredTexturesLayout>, SRes<LightsFeatureBuffer>);

    fn describe(
        (render_pipeline, render_pass_mesh, camera_feature, deferred_textures_layout, lights_buffer): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(Some(render_pipeline.iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::Mesh(render_pass_mesh.deferred_mesh.as_ref().map(|m| m.id())),
            SubPassCommand::BindGroup(0, camera_feature.bind_group.clone()),
            SubPassCommand::BindGroup(1, deferred_textures_layout.deferred_bind_group_resolved.clone()),
            SubPassCommand::BindGroup(2, lights_buffer.bind_group.clone()),
            SubPassCommand::DrawBatches(vec![DrawCommandsBatch {
                bind_group: None,
                index_range: 0..6,
                instance_range: 0..1
            }])
        ])
    }

    fn label() -> &'static str { "lighting-pbr-main" }
}
