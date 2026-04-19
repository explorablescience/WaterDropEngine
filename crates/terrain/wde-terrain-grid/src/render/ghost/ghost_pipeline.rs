use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_camera::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

use crate::render::ghost::{GhostMaterial, ghost_subpass::PushConstants};

#[derive(TypePath, Asset, Default, Clone)]
pub struct GhostRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for GhostRenderPipeline {
    type SourceAsset = RenderPipelineAsset<GhostRenderPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<CameraBinding>,
        SBinding<SsboMeshBinding>,
        SBinding<SsboTransformBinding>,
        SBinding<GhostMaterial>
    );

    fn prepare(
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, camera, ssbo_mesh, ssbo_transform, materials): &mut SystemParamItem<
            Self::Params,
        >
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(GhostRenderPipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "ghost",
                    vert: Some(assets_server.load("core/render/ghost/ghost_vert.wgsl")),
                    frag: Some(assets_server.load("core/render/ghost/ghost_frag.wgsl")),
                    bind_group_layouts: vec![
                        ssbo_mesh.iter().next().map(|(_, m)| m.layout.clone()),
                        camera.iter().next().map(|(_, c)| c.layout.clone()),
                        ssbo_transform.iter().next().map(|(_, t)| t.layout.clone()),
                        materials.iter().next().map(|(_, m)| m.layout.clone()),
                    ],
                    depth: DepthDescriptor {
                        enabled: true,
                        ..default()
                    },
                    sample_count: MSAA_SAMPLE_COUNT,
                    push_constants: vec![PushConstantDescriptor {
                        stages: ShaderStages::VERTEX,
                        offset: 0,
                        size: std::mem::size_of::<PushConstants>() as u32
                    }],
                    vertex_buffer: false,
                    fragment_blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add
                        }
                    }),
                    ..default()
                },
                asset
            )?
        ))
    }
}
