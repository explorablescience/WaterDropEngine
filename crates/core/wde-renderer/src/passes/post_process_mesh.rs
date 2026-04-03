use bevy::prelude::*;

use crate::prelude::*;

#[derive(Resource, Default)]
pub struct PostProcessingMesh(pub Option<Handle<MeshAsset>>);
impl PostProcessingMesh {
    pub fn init(assets_server: Res<AssetServer>, mut mesh: ResMut<PostProcessingMesh>) {
        let deferred_mesh: Handle<MeshAsset> = assets_server.add(MeshAsset {
            label: "post-process-mesh".to_string(),
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
        mesh.0 = Some(deferred_mesh);
    }

    pub fn extract(pass_main: ExtractWorld<Res<PostProcessingMesh>>, mut pass_render: ResMut<PostProcessingMesh>) {
        pass_render.0 = None;
        if let Some(ref mesh_cpu) = pass_main.0 {
            pass_render.0 = Some(mesh_cpu.clone());
        }
    }
}
