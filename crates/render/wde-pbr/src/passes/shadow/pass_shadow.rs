use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_logger::prelude::*;
use wde_renderer::prelude::*;

use crate::{
    deferred::{BatchList, SsboTransformBinding},
    passes::shadow::{CascadeEnabledFlags, ShadowMapData, ShadowParamsBinding}
};

pub struct RenderPassShadow;
impl RenderPass for RenderPassShadow {
    type Params = SRenderData<ShadowMapData>;

    fn describe(shadow_data: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc {
            attachments_colors: Some(vec![]), // depth-only pass; empty prevents swapchain fallback
            attachments_depth: Some(RenderPassDescDepthAttachment {
                texture: shadow_data
                    .iter()
                    .next()
                    .and_then(|(_, d)| d.get_texture(ShadowMapData::SHADOW_MAP_IDX))
                    .map(|t| t.id()),
                load: LoadOp::Clear(1.0),
                ..default()
            })
        }
    }

    fn id() -> RenderPassId {
        5
    }

    fn label() -> &'static str {
        "shadow-map"
    }
}

#[derive(TypePath, Default, Clone, Debug)]
pub struct ShadowMapPipeline(pub CachedPipelineIndex);
impl RenderAsset for ShadowMapPipeline {
    type SourceAsset = RenderPipelineAsset<ShadowMapPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<SsboMeshBinding>,
        SBinding<ShadowParamsBinding>,
        SBinding<SsboTransformBinding>
    );

    fn prepare(
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, ssbo_mesh, shadow_params, ssbo_transform): &mut SystemParamItem<
            Self::Params
        >
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(ShadowMapPipeline(pipeline_manager.create_render_pipeline(
            RenderPipelineDescriptor {
                label: "shadow-map",
                vert: Some(assets_server.load::<Shader>("core/render/pbr/shadow_map.vert.wgsl")),
                frag: Some(assets_server.load::<Shader>("core/render/pbr/shadow_map.frag.wgsl")),
                bind_group_layouts: vec![
                    ssbo_mesh.iter().next().map(|(_, m)| m.layout.clone()),
                    shadow_params.iter().next().map(|(_, s)| s.layout.clone()),
                    ssbo_transform.iter().next().map(|(_, t)| t.layout.clone()),
                ],
                depth: DepthDescriptor {
                    enabled: true,
                    write: true,
                    compare: CompareFunction::Less,
                    format: Some(TextureFormat::Depth32Float),
                    ..default()
                },
                render_targets: Some(vec![]),
                cull_mode: Some(Face::Front),
                vertex_buffer: false,
                push_constants: vec![PushConstantDescriptor {
                    stages: ShaderStages::VERTEX,
                    offset: 0,
                    size: std::mem::size_of::<ShadowPushConstants>() as u32
                }],
                ..default()
            },
            asset
        )?))
    }
}

pub struct SubRenderPassShadow;
impl RenderSubPass for SubRenderPassShadow {
    type Params = (
        SRes<RenderAssets<ShadowMapPipeline>>,
        SBinding<SsboMeshBinding>,
        SBinding<ShadowParamsBinding>,
        SBinding<SsboTransformBinding>
    );

    fn describe(
        (pipeline, ssbo_mesh, shadow_params, ssbo_transform): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(Some(pipeline.iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::BindGroup(
                0,
                ssbo_mesh.iter().next().map(|(_, m)| m.bind_group.clone())
            ),
            SubPassCommand::BindGroup(
                1,
                shadow_params
                    .iter()
                    .next()
                    .map(|(_, s)| s.bind_group.clone())
            ),
            SubPassCommand::BindGroup(
                2,
                ssbo_transform
                    .iter()
                    .next()
                    .map(|(_, t)| t.bind_group.clone())
            ),
            SubPassCommand::Custom(draw_shadow_custom),
        ])
    }

    fn label() -> &'static str {
        "shadow-map"
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ShadowPushConstants {
    first_vertex: u32,
    first_index: u32,
    cascade_index: u32
}

fn draw_shadow_custom<'pass>(world: &'pass World, render_pass: &mut RenderPassInstance<'pass>) {
    let flags = world
        .get_resource::<CascadeEnabledFlags>()
        .map(|f| f.0)
        .unwrap_or([true; 3]);
    if !flags[0] {
        return;
    }
    draw_shadow_cascade(world, render_pass, 0);
}

fn draw_shadow_cascade_1<'pass>(world: &'pass World, render_pass: &mut RenderPassInstance<'pass>) {
    let flags = world
        .get_resource::<CascadeEnabledFlags>()
        .map(|f| f.0)
        .unwrap_or([true; 3]);
    if !flags[1] {
        return;
    }
    draw_shadow_cascade(world, render_pass, 1);
}

fn draw_shadow_cascade_2<'pass>(world: &'pass World, render_pass: &mut RenderPassInstance<'pass>) {
    let flags = world
        .get_resource::<CascadeEnabledFlags>()
        .map(|f| f.0)
        .unwrap_or([true; 3]);
    if !flags[2] {
        return;
    }
    draw_shadow_cascade(world, render_pass, 2);
}

fn draw_shadow_cascade<'pass>(
    world: &'pass World,
    render_pass: &mut RenderPassInstance<'pass>,
    cascade_idx: u32
) {
    let batches = world.get_resource::<BatchList>().unwrap();
    let meshes = world.get_resource::<RenderAssets<GpuMesh>>().unwrap();

    let mut current_mesh_id = None;
    let mut current_index_count = 0;
    for batch_key in batches.sorted_batches.iter() {
        if !batches.shadow_casting_batches.contains(batch_key) {
            continue;
        }
        let Some(batch) = batches.batches.get(batch_key) else {
            continue;
        };
        if current_mesh_id != Some(batch_key.mesh) {
            let Some(mesh) = meshes.get(batch_key.mesh) else {
                continue;
            };
            current_index_count = mesh.index_count;
            current_mesh_id = Some(batch_key.mesh);
            render_pass.set_push_constants(
                ShaderStages::VERTEX,
                bytemuck::bytes_of(&[ShadowPushConstants {
                    first_vertex: mesh.ssbo_first_vertex,
                    first_index: mesh.ssbo_first_index,
                    cascade_index: cascade_idx
                }])
            );
        }
        if let Err(e) = render_pass.draw(
            0..current_index_count,
            batch.instances_offset
                ..(batch.instances_offset + batch.transform_ssbo_ids.len() as u32)
        ) {
            error!("Shadow draw failed: {:?}.", e);
        }
    }
}

// ============== CASCADE 1 ==============

pub struct RenderPassShadow1;
impl RenderPass for RenderPassShadow1 {
    type Params = SRenderData<ShadowMapData>;

    fn describe(shadow_data: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc {
            attachments_colors: Some(vec![]),
            attachments_depth: Some(RenderPassDescDepthAttachment {
                texture: shadow_data
                    .iter()
                    .next()
                    .and_then(|(_, d)| d.get_texture(ShadowMapData::SHADOW_MAP_CASCADE_1))
                    .map(|t| t.id()),
                load: LoadOp::Clear(1.0),
                ..default()
            })
        }
    }

    fn id() -> RenderPassId {
        6
    }

    fn label() -> &'static str {
        "shadow-map-cascade-1"
    }
}

#[derive(TypePath, Default, Clone, Debug)]
pub struct ShadowMapPipeline1(pub CachedPipelineIndex);
impl RenderAsset for ShadowMapPipeline1 {
    type SourceAsset = RenderPipelineAsset<ShadowMapPipeline1>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<SsboMeshBinding>,
        SBinding<ShadowParamsBinding>,
        SBinding<SsboTransformBinding>
    );

    fn prepare(
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, ssbo_mesh, shadow_params, ssbo_transform): &mut SystemParamItem<
            Self::Params
        >
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(ShadowMapPipeline1(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "shadow-map-cascade-1",
                    vert: Some(
                        assets_server.load::<Shader>("core/render/pbr/shadow_map.vert.wgsl")
                    ),
                    frag: Some(
                        assets_server.load::<Shader>("core/render/pbr/shadow_map.frag.wgsl")
                    ),
                    bind_group_layouts: vec![
                        ssbo_mesh.iter().next().map(|(_, m)| m.layout.clone()),
                        shadow_params.iter().next().map(|(_, s)| s.layout.clone()),
                        ssbo_transform.iter().next().map(|(_, t)| t.layout.clone()),
                    ],
                    depth: DepthDescriptor {
                        enabled: true,
                        write: true,
                        compare: CompareFunction::Less,
                        format: Some(TextureFormat::Depth32Float),
                        ..default()
                    },
                    render_targets: Some(vec![]),
                    cull_mode: Some(Face::Front),
                    vertex_buffer: false,
                    push_constants: vec![PushConstantDescriptor {
                        stages: ShaderStages::VERTEX,
                        offset: 0,
                        size: std::mem::size_of::<ShadowPushConstants>() as u32
                    }],
                    ..default()
                },
                asset
            )?
        ))
    }
}

pub struct SubRenderPassShadow1;
impl RenderSubPass for SubRenderPassShadow1 {
    type Params = (
        SRes<RenderAssets<ShadowMapPipeline1>>,
        SBinding<SsboMeshBinding>,
        SBinding<ShadowParamsBinding>,
        SBinding<SsboTransformBinding>
    );

    fn describe(
        (pipeline, ssbo_mesh, shadow_params, ssbo_transform): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(Some(pipeline.iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::BindGroup(
                0,
                ssbo_mesh.iter().next().map(|(_, m)| m.bind_group.clone())
            ),
            SubPassCommand::BindGroup(
                1,
                shadow_params
                    .iter()
                    .next()
                    .map(|(_, s)| s.bind_group.clone())
            ),
            SubPassCommand::BindGroup(
                2,
                ssbo_transform
                    .iter()
                    .next()
                    .map(|(_, t)| t.bind_group.clone())
            ),
            SubPassCommand::Custom(draw_shadow_cascade_1),
        ])
    }

    fn label() -> &'static str {
        "shadow-map-cascade-1"
    }
}

// ============== CASCADE 2 ==============

pub struct RenderPassShadow2;
impl RenderPass for RenderPassShadow2 {
    type Params = SRenderData<ShadowMapData>;

    fn describe(shadow_data: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc {
            attachments_colors: Some(vec![]),
            attachments_depth: Some(RenderPassDescDepthAttachment {
                texture: shadow_data
                    .iter()
                    .next()
                    .and_then(|(_, d)| d.get_texture(ShadowMapData::SHADOW_MAP_CASCADE_2))
                    .map(|t| t.id()),
                load: LoadOp::Clear(1.0),
                ..default()
            })
        }
    }

    fn id() -> RenderPassId {
        7
    }

    fn label() -> &'static str {
        "shadow-map-cascade-2"
    }
}

#[derive(TypePath, Default, Clone, Debug)]
pub struct ShadowMapPipeline2(pub CachedPipelineIndex);
impl RenderAsset for ShadowMapPipeline2 {
    type SourceAsset = RenderPipelineAsset<ShadowMapPipeline2>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<SsboMeshBinding>,
        SBinding<ShadowParamsBinding>,
        SBinding<SsboTransformBinding>
    );

    fn prepare(
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, ssbo_mesh, shadow_params, ssbo_transform): &mut SystemParamItem<
            Self::Params
        >
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(ShadowMapPipeline2(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "shadow-map-cascade-2",
                    vert: Some(
                        assets_server.load::<Shader>("core/render/pbr/shadow_map.vert.wgsl")
                    ),
                    frag: Some(
                        assets_server.load::<Shader>("core/render/pbr/shadow_map.frag.wgsl")
                    ),
                    bind_group_layouts: vec![
                        ssbo_mesh.iter().next().map(|(_, m)| m.layout.clone()),
                        shadow_params.iter().next().map(|(_, s)| s.layout.clone()),
                        ssbo_transform.iter().next().map(|(_, t)| t.layout.clone()),
                    ],
                    depth: DepthDescriptor {
                        enabled: true,
                        write: true,
                        compare: CompareFunction::Less,
                        format: Some(TextureFormat::Depth32Float),
                        ..default()
                    },
                    render_targets: Some(vec![]),
                    cull_mode: Some(Face::Front),
                    vertex_buffer: false,
                    push_constants: vec![PushConstantDescriptor {
                        stages: ShaderStages::VERTEX,
                        offset: 0,
                        size: std::mem::size_of::<ShadowPushConstants>() as u32
                    }],
                    ..default()
                },
                asset
            )?
        ))
    }
}

pub struct SubRenderPassShadow2;
impl RenderSubPass for SubRenderPassShadow2 {
    type Params = (
        SRes<RenderAssets<ShadowMapPipeline2>>,
        SBinding<SsboMeshBinding>,
        SBinding<ShadowParamsBinding>,
        SBinding<SsboTransformBinding>
    );

    fn describe(
        (pipeline, ssbo_mesh, shadow_params, ssbo_transform): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(Some(pipeline.iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::BindGroup(
                0,
                ssbo_mesh.iter().next().map(|(_, m)| m.bind_group.clone())
            ),
            SubPassCommand::BindGroup(
                1,
                shadow_params
                    .iter()
                    .next()
                    .map(|(_, s)| s.bind_group.clone())
            ),
            SubPassCommand::BindGroup(
                2,
                ssbo_transform
                    .iter()
                    .next()
                    .map(|(_, t)| t.bind_group.clone())
            ),
            SubPassCommand::Custom(draw_shadow_cascade_2),
        ])
    }

    fn label() -> &'static str {
        "shadow-map-cascade-2"
    }
}
