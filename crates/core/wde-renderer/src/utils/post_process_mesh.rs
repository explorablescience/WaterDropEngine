use bevy::prelude::*;

use crate::prelude::*;

/// Resource storing the handle to the full-screen quad mesh used for post-processing passes.
/// This mesh is a simple quad covering the entire screen, with UVs for sampling the rendered texture.
#[derive(Resource, Default)]
pub struct PostProcessingMesh(pub Option<Handle<Mesh>>);
impl PostProcessingMesh {
    pub fn init(assets_server: Res<AssetServer>, mut mesh: ResMut<PostProcessingMesh>) {
        let deferred_mesh: Handle<Mesh> = assets_server.add(Mesh {
            label: "post-process-mesh".to_string(),
            vertices: vec![
                Vertex {
                    position: [-1.0, 1.0, 0.0],
                    uv: [0.0, 1.0],
                    ..Default::default()
                },
                Vertex {
                    position: [-1.0, -1.0, 0.0],
                    uv: [0.0, 0.0],
                    ..Default::default()
                },
                Vertex {
                    position: [1.0, -1.0, 0.0],
                    uv: [1.0, 0.0],
                    ..Default::default()
                },
                Vertex {
                    position: [1.0, 1.0, 0.0],
                    uv: [1.0, 1.0],
                    ..Default::default()
                },
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            bbox: MeshBbox {
                min: Vec3::new(-1.0, -1.0, 0.0),
                max: Vec3::new(1.0, 1.0, 0.0)
            },
            use_ssbo: false
        });
        mesh.0 = Some(deferred_mesh);
    }
}
impl ExtractResource for PostProcessingMesh {
    type Source = Self;

    fn extract(source: &Self::Source) -> Self {
        Self(source.0.clone())
    }
}
