// use bevy::{ecs::system::{SystemParamItem, lifetimeless::{SRes, SResMut}}, prelude::*};
// use wde_camera::prelude::*;
// use wde_pbr::prelude::*;
// use wde_renderer::{prelude::*, ssbos::ssbo_mesh::SsboMesh};

// use crate::render::ghost::GhostMaterial;

// #[derive(Default, Asset, Clone, TypePath)]
// pub struct GhostRenderPipelineAsset;
// #[allow(unused)]
// #[derive(Component)]
// pub struct GhostRenderPipeline(pub Handle<GhostRenderPipelineAsset>);
// pub struct GpuGhostRenderPipeline(pub CachedPipelineIndex);
// impl RenderAsset for GpuGhostRenderPipeline {
//     type SourceAsset = GhostRenderPipelineAsset;
//     type Params = (SRes<AssetServer>, SResMut<PipelineManager>, SRes<CameraFeatureRender>, SMaterial<GhostMaterial>);

//     fn prepare(
//             asset: Self::SourceAsset,
//             (assets_server, pipeline_manager, camera_feature, materials): &mut SystemParamItem<Self::Params>
//         ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
//         let material = match materials.iter().next() {
//             Some((_, material)) => material,
//             None => return Err(PrepareAssetError::RetryNextUpdate(asset))
//         };

//         Ok(GpuGhostRenderPipeline(pipeline_manager.create_render_pipeline(RenderPipelineDescriptor {
//             label: "ghost",
//             vert: Some(assets_server.load("core/render/ghost/ghost_vert.wgsl")),
//             frag: Some(assets_server.load("core/render/ghost/ghost_frag.wgsl")),
//             bind_group_layouts: vec![
//                 SsboMesh::layout(),
//                 camera_feature.layout.clone(),
//                 SsboTransformPbr::get_layout(),
//                 material.layout.clone()
//             ],
//             depth: DepthDescriptor {
//                 enabled: false,
//                 ..default()
//             },
//             sample_count: MSAA_SAMPLE_COUNT,
//             ..default()
//         })))
//     }
// }
