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

use crate::material::OutlineMaterial;

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct OutlineRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for OutlineRenderPipeline {
    type SourceAsset = RenderPipelineAsset<OutlineRenderPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<SsboMeshBinding>,
        SBinding<CameraBinding>,
        SBinding<SsboTransformBinding>,
        SBinding<OutlineMaterial>
    );

    fn prepare(
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, ssbo_mesh, camera, transforms, materials): &mut SystemParamItem<
            Self::Params,
        >
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(OutlineRenderPipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "outline-render",
                    vert: Some(assets_server.load("core/render/pbr-outline/outline.vert.wgsl")),
                    frag: Some(assets_server.load("core/render/pbr-outline/outline.frag.wgsl")),
                    bind_group_layouts: vec![
                        ssbo_mesh.iter().next().map(|(_, m)| m.layout.clone()),
                        camera.iter().next().map(|(_, c)| c.layout.clone()),
                        transforms.iter().next().map(|(_, t)| t.layout.clone()),
                        materials.iter().next().map(|(_, m)| m.layout.clone()),
                    ],
                    depth: DepthDescriptor {
                        enabled: true,
                        write: false,
                        compare: CompareFunction::NotEqual,
                        stencil: StencilState {
                            front: StencilFaceState {
                                compare: CompareFunction::NotEqual,
                                fail_op: StencilOperation::Keep,
                                depth_fail_op: StencilOperation::Keep,
                                pass_op: StencilOperation::Keep
                            },
                            back: StencilFaceState {
                                compare: CompareFunction::NotEqual,
                                fail_op: StencilOperation::Keep,
                                depth_fail_op: StencilOperation::Keep,
                                pass_op: StencilOperation::Keep
                            },
                            read_mask: 0xff,
                            write_mask: 0x00
                        }
                    },
                    fragment_blend: Some(BlendState::ALPHA_BLENDING),
                    cull_mode: None,
                    push_constants: vec![PushConstantDescriptor {
                        stages: ShaderStages::VERTEX,
                        offset: 0,
                        size: std::mem::size_of::<PushConstants>() as u32
                    }],
                    sample_count: MSAA_SAMPLE_COUNT,
                    vertex_buffer: false,
                    ..default()
                },
                asset
            )?
        ))
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct PushConstants {
    pub first_vertex: u32,
    pub first_index: u32,
    pub transform_id: u32
}
