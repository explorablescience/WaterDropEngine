use bevy::prelude::*;
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::{logic::{lights::LightsFeatureBuffer, textures::PbrDeferredTexturesLayout}, passes::pipeline_lighting::GpuPbrLightingRenderPipeline};


#[derive(Resource, Default)]
pub(crate) struct PbrLightingRenderPassMesh {
    pub deferred_mesh: Option<Handle<MeshAsset>>
}
impl PbrLightingRenderPassMesh {
    // Creates the rendering mesh.
    pub fn init(assets_server: Res<AssetServer>, mut render_pass: ResMut<PbrLightingRenderPassMesh>) {
        // Create the 2d quad mesh
        let deferred_mesh: Handle<MeshAsset> = assets_server.add(MeshAsset {
            label: "deferred-lighting-pass".to_string(),
            vertices: vec![
                Vertex { position: [-1.0, 1.0, 0.0], uv: [0.0, 1.0], ..Default::default() },
                Vertex { position: [-1.0, -1.0, 0.0], uv: [0.0, 0.0], ..Default::default() },
                Vertex { position: [1.0, -1.0, 0.0], uv: [1.0, 0.0], ..Default::default() },
                Vertex { position: [1.0, 1.0, 0.0], uv: [1.0, 1.0], ..Default::default() },
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            bounding_box: ModelBoundingBox {
                min: Vec3::new(-1.0, -1.0, 0.0),
                max: Vec3::new(1.0, 1.0, 0.0),
            },
            use_ssbo: false
        });
        render_pass.deferred_mesh = Some(deferred_mesh);
    }

    pub fn extract(
        pass_main: ExtractWorld<Res<PbrLightingRenderPassMesh>>,
        mut pass_render: ResMut<PbrLightingRenderPassMesh>,
    ) {
        pass_render.deferred_mesh = None;
        if let Some(ref mesh_cpu) = pass_main.deferred_mesh {
            pass_render.deferred_mesh = Some(mesh_cpu.clone());
        }
    }
}

#[derive(Resource, Default)]
pub struct PbrLightingRenderPass;
impl RenderPass for PbrLightingRenderPass {
    fn render(&self, world: &mut World) {
        let sub_pass_desc = SubPassDesc(vec![
            SubPassCommand::Pipeline(Some(world.get_resource::<RenderAssets<GpuPbrLightingRenderPipeline>>().unwrap().iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::Mesh(world.get_resource::<PbrLightingRenderPassMesh>().unwrap().deferred_mesh.as_ref().map(|m| m.id())),
            SubPassCommand::BindGroup(0, world.get_resource::<CameraFeatureRender>().unwrap().bind_group.clone()),
            SubPassCommand::BindGroup(1, world.get_resource::<PbrDeferredTexturesLayout>().unwrap().deferred_bind_group_resolved.clone()),
            SubPassCommand::BindGroup(2, world.get_resource::<LightsFeatureBuffer>().unwrap().bind_group.clone()),
            SubPassCommand::DrawBatches(vec![DrawCommandsBatch {
                bind_group: None,
                index_range: 0..6,
                instance_range: 0..1
            }])
        ]);
        self.process(world, &RenderPassDesc::default(), &sub_pass_desc);
    }

    fn label(&self) -> &str {
        "pbr-lighting"
    }
}
