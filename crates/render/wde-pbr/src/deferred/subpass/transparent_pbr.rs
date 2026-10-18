use wde_logger::prelude::*;

use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::{
    deferred::subpass::gbuffer_bindgroup::SsboTransformBinding,
    prelude::{BatchList, LightsData, PbrMaterial, is_batch_blended}
};

/// Camera and lighting data in a single group, keeping the pipeline within 4 bind groups.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct TransparentSceneBinding;
impl RenderBinding for TransparentSceneBinding {
    type Params = (SRenderData<CameraRender>, SRenderData<LightsData>);

    fn describe(
        &mut self,
        (camera, lights): &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder
            .add_buffer(camera, CameraRender::CAMERA_IDX)
            .add_buffer(lights, LightsData::LIGHTS_BUFFER_IDX);
        #[cfg(feature = "atmosphere")]
        builder.add_buffer(lights, LightsData::ATMOSPHERE_PARAMS_BUFFER_IDX);
        builder.add_buffer(lights, LightsData::PBR_PARAMS_BUFFER_IDX);
    }

    fn label(&self) -> &str {
        "pbr-transparent-scene"
    }
}

/// Forward-lit, alpha-blended pipeline for [PbrMaterial]s using [PbrAlphaMode::Blend](crate::prelude::PbrAlphaMode::Blend).
#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct TransparentPbrRenderPipeline(pub CachedPipelineIndex);
impl RenderAsset for TransparentPbrRenderPipeline {
    type SourceAsset = RenderPipelineAsset<TransparentPbrRenderPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<SsboMeshBinding>,
        SBinding<TransparentSceneBinding>,
        SBinding<SsboTransformBinding>,
        SBinding<PbrMaterial>
    );

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        (asset_server, pipeline_manager, ssbo_mesh, scene, transforms, materials): &mut SystemParamItem<
            Self::Params,
        >
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(TransparentPbrRenderPipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "pbr-transparent",
                    vert: Some(asset_server.load("core/render/pbr/transparent.vert.wgsl")),
                    frag: Some(asset_server.load("core/render/pbr/transparent.frag.wgsl")),
                    bind_group_layouts: vec![
                        ssbo_mesh.iter().next().map(|(_, m)| m.layout.clone()),
                        scene.iter().next().map(|(_, s)| s.layout.clone()),
                        transforms.iter().next().map(|(_, t)| t.layout.clone()),
                        materials.iter().next().map(|(_, m)| m.layout.clone()),
                    ],
                    // Tested against the opaque depth, but not written so glass layers all blend
                    depth: DepthDescriptor {
                        enabled: true,
                        write: false,
                        compare: CompareFunction::Less,
                        stencil: StencilState::default(),
                        format: None
                    },
                    fragment_blend: Some(BlendState::ALPHA_BLENDING),
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
struct PushConstants {
    first_vertex: u32,
    first_index: u32
}

pub(crate) struct SubRenderPassTransparentPbr;
impl RenderSubPass for SubRenderPassTransparentPbr {
    type Params = (
        SRes<RenderAssets<TransparentPbrRenderPipeline>>,
        SBinding<SsboMeshBinding>,
        SBinding<TransparentSceneBinding>,
        SBinding<SsboTransformBinding>
    );

    fn describe(
        (pipeline, ssbo_mesh, scene, transforms): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(Some(pipeline.iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::BindGroup(
                0,
                ssbo_mesh.iter().next().map(|(_, m)| m.bind_group.clone())
            ),
            SubPassCommand::BindGroup(1, scene.iter().next().map(|(_, s)| s.bind_group.clone())),
            SubPassCommand::BindGroup(
                2,
                transforms.iter().next().map(|(_, t)| t.bind_group.clone())
            ),
            SubPassCommand::Custom(draw_transparent),
        ])
    }

    fn label() -> &'static str {
        "pbr-transparent"
    }
}

fn draw_transparent<'pass>(world: &'pass World, render_pass: &mut RenderPassInstance<'pass>) {
    let batches = world.get_resource::<BatchList>().unwrap();
    let materials = world
        .get_resource::<RenderAssets<GpuRenderBinding<PbrMaterial>>>()
        .unwrap();
    let meshes = world.get_resource::<RenderAssets<GpuMesh>>().unwrap();

    for batch_key in batches.sorted_batches.iter() {
        if !is_batch_blended(materials, batch_key) {
            continue;
        }
        let (Some(batch), Some(mesh), Some(material)) = (
            batches.batches.get(batch_key),
            meshes.get(batch_key.mesh),
            materials.get(batch_key.material)
        ) else {
            continue;
        };
        render_pass.set_push_constants(
            ShaderStages::VERTEX,
            bytemuck::bytes_of(&PushConstants {
                first_vertex: mesh.ssbo_first_vertex,
                first_index: mesh.ssbo_first_index
            })
        );
        render_pass.set_bind_group(3, &material.bind_group);
        if let Err(e) = render_pass.draw(
            0..mesh.index_count,
            batch.instances_offset
                ..(batch.instances_offset + batch.transform_ssbo_ids.len() as u32)
        ) {
            error!("Failed to draw transparent batch: {:?}.", e);
        }
    }
}
