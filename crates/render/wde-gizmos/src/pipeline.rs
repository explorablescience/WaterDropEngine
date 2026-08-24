use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::data::{GizmoLinesBinding, GizmoQuadsBinding};

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct GizmoRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for GizmoRenderPipeline {
    type SourceAsset = RenderPipelineAsset<GizmoRenderPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<CameraBinding>,
        SBinding<GizmoLinesBinding>
    );

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, camera, gizmo_lines): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(GizmoRenderPipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "gizmos-render",
                    vert: Some(assets_server.load("core/render/gizmo/vert.wgsl")),
                    frag: Some(assets_server.load("core/render/gizmo/frag.wgsl")),
                    bind_group_layouts: vec![
                        camera.iter().next().map(|(_, c)| c.layout.clone()),
                        gizmo_lines.iter().next().map(|(_, g)| g.layout.clone()),
                    ],
                    // Depth-tested against the scene so gizmos are occluded by geometry, but no
                    // depth write since they are a debug overlay rather than scene geometry.
                    depth: DepthDescriptor {
                        enabled: true,
                        write: false,
                        compare: CompareFunction::Less,
                        stencil: StencilState::default(),
                        format: None
                    },
                    fragment_blend: Some(BlendState::ALPHA_BLENDING),
                    cull_mode: None,
                    topology: RenderTopology::LineList,
                    sample_count: MSAA_SAMPLE_COUNT,
                    // Vertices are pulled from the gizmo lines storage buffer by index, not read
                    // from a hardware vertex buffer.
                    vertex_buffer: false,
                    ..default()
                },
                asset
            )?
        ))
    }
}

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct GizmoQuadRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for GizmoQuadRenderPipeline {
    type SourceAsset = RenderPipelineAsset<GizmoQuadRenderPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<CameraBinding>,
        SBinding<GizmoQuadsBinding>
    );

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, camera, gizmo_quads): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(GizmoQuadRenderPipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "gizmos-quad-render",
                    // Same vertex-pulling shaders as the line pipeline; only the topology differs.
                    vert: Some(assets_server.load("core/render/gizmo/vert.wgsl")),
                    frag: Some(assets_server.load("core/render/gizmo/frag.wgsl")),
                    bind_group_layouts: vec![
                        camera.iter().next().map(|(_, c)| c.layout.clone()),
                        gizmo_quads.iter().next().map(|(_, g)| g.layout.clone()),
                    ],
                    depth: DepthDescriptor {
                        enabled: true,
                        write: false,
                        compare: CompareFunction::Less,
                        stencil: StencilState::default(),
                        format: None
                    },
                    fragment_blend: Some(BlendState::ALPHA_BLENDING),
                    cull_mode: None,
                    topology: RenderTopology::TriangleList,
                    sample_count: MSAA_SAMPLE_COUNT,
                    vertex_buffer: false,
                    ..default()
                },
                asset
            )?
        ))
    }
}
