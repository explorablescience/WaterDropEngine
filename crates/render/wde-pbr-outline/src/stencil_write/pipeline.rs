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

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct StencilWriteRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for StencilWriteRenderPipeline {
    type SourceAsset = RenderPipelineAsset<StencilWriteRenderPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<SsboMeshBinding>,
        SBinding<CameraBinding>,
        SBinding<SsboTransformBinding>
    );

    fn prepare(
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, ssbo_mesh, camera, transforms): &mut SystemParamItem<
            Self::Params
        >
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(StencilWriteRenderPipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "stencil-write-pipeline",
                    vert: Some(
                        assets_server.load("core/render/pbr-outline/stencil_write.vert.wgsl")
                    ),
                    frag: Some(
                        assets_server.load("core/render/pbr-outline/stencil_write.frag.wgsl")
                    ),
                    bind_group_layouts: vec![
                        ssbo_mesh.iter().next().map(|(_, m)| m.layout.clone()),
                        camera.iter().next().map(|(_, c)| c.layout.clone()),
                        transforms.iter().next().map(|(_, t)| t.layout.clone()),
                    ],
                    depth: DepthDescriptor {
                        enabled: true,
                        write: false,
                        compare: CompareFunction::Always,
                        stencil: StencilState {
                            front: StencilFaceState {
                                compare: CompareFunction::Always,
                                fail_op: StencilOperation::Keep,
                                depth_fail_op: StencilOperation::Keep,
                                pass_op: StencilOperation::Replace
                            },
                            back: StencilFaceState {
                                compare: CompareFunction::Always,
                                fail_op: StencilOperation::Keep,
                                depth_fail_op: StencilOperation::Keep,
                                pass_op: StencilOperation::Replace
                            },
                            read_mask: 0x00,
                            write_mask: 0xff
                        }
                    },
                    render_targets: Some(vec![
                        TextureFormat::R16Float,       // Depth
                        TextureFormat::Rgba8UnormSrgb, // Albedo
                        TextureFormat::Rgba16Float,    // Normal
                        TextureFormat::R8Unorm,        // AO
                    ]),
                    cull_mode: None,
                    color_write: ColorWrites::empty(),
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
