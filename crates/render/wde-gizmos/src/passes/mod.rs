use bevy::prelude::*;

mod gizmo_pipeline;
mod gizmo_pass;
mod gizmo_subpass;
mod gizmo_ssbo;

use gizmo_pipeline::*;
use gizmo_subpass::*;
use gizmo_ssbo::*;
use wde_renderer::prelude::*;

use crate::passes::gizmo_pass::GizmoRenderPass;

pub(crate) struct GizmoFeaturesPlugin;
impl Plugin for GizmoFeaturesPlugin {
    fn build(&self, app: &mut App) {
        // Add the gizmo ssbo
        app
            .add_plugins(GizmoSsboPlugin);

        // Add the pbr pipelines
        app
            .init_asset::<GizmoRenderPipelineAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuGizmoRenderPipeline>::default());

        // Add the extract system for the render pass
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Extract, GizmoRenderSubPass::extract);

        // Always add the gizmo render pass (at the end)
        app.get_sub_app_mut(RenderApp).unwrap()
            .world_mut().get_resource_mut::<RenderGraph>().unwrap()
            .add_pass::<GizmoRenderPass>()
            .add_sub_pass::<GizmoRenderSubPass, GizmoRenderPass>();
    }

    fn finish(&self, app: &mut App) {
        // Create the render pass
        app.get_sub_app_mut(RenderApp).unwrap()
            .insert_resource(GizmoRenderSubPass {
                batches_order: Default::default(),
                batches: Default::default()
            });

        // Create the gizmo pipeline
        let pipeline: Handle<GizmoRenderPipelineAsset> = app.world_mut()
            .get_resource::<AssetServer>().unwrap().add(GizmoRenderPipelineAsset);
        app.get_sub_app_mut(RenderApp).unwrap().world_mut().spawn(GizmoRenderPipeline(pipeline));
    }
}

