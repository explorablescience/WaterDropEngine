use bevy::{ecs::system::SystemParamItem, prelude::*};
use wde_renderer::prelude::*;


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

pub struct PbrLightingRenderPass;
impl RenderPass for PbrLightingRenderPass {
    type Params = ();

    fn describe(_params: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc::default()
    }

    fn id() -> RenderPassId { 51 }
    fn label() -> &'static str { "pbr-lighting" }
}
