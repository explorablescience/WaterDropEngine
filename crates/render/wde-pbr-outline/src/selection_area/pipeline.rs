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

use crate::selection_area::material::SelectionAreaMaterial;

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct SelectionAreaRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for SelectionAreaRenderPipeline {
    type SourceAsset = RenderPipelineAsset<SelectionAreaRenderPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<SsboMeshBinding>,
        SBinding<CameraBinding>,
        SBinding<SsboTransformBinding>,
        SBinding<SelectionAreaMaterial>
    );

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, ssbo_mesh, camera, transforms, materials): &mut SystemParamItem<
            Self::Params,
        >
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(SelectionAreaRenderPipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "selection-area-render",
                    vert: Some(
                        assets_server.load("core/render/pbr-outline/selection_area.vert.wgsl")
                    ),
                    frag: Some(
                        assets_server.load("core/render/pbr-outline/selection_area.frag.wgsl")
                    ),
                    bind_group_layouts: vec![
                        ssbo_mesh.iter().next().map(|(_, m)| m.layout.clone()),
                        camera.iter().next().map(|(_, c)| c.layout.clone()),
                        transforms.iter().next().map(|(_, t)| t.layout.clone()),
                        materials.iter().next().map(|(_, m)| m.layout.clone()),
                    ],
                    // Drawn over everything (no depth test, no depth write): the decal must stay
                    // visible on building floors, e.g. inside domes and tunnels
                    depth: DepthDescriptor {
                        enabled: true,
                        write: false,
                        compare: CompareFunction::Always,
                        stencil: StencilState::default(),
                        format: None
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
