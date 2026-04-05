use bevy::{ecs::system::{SystemParamItem, lifetimeless::SRes}, prelude::*};
use wde_renderer::prelude::*;
use wde_camera::prelude::*;
use wde_terrain::prelude::CHUNK_COUNT;

use crate::{editor::PlacementUI, render::grid::{buffers::TerrainGridBuffer, terrain_grid_pipeline::GpuTerrainGridRenderPipeline}};

#[derive(Resource, Default)]
pub struct RenderSubPassTerrainGrid {
    mesh: Option<Handle<Mesh>>,
    render_grid: bool
}
impl RenderSubPassTerrainGrid {
    pub fn init(assets_server: Res<AssetServer>, mut pass: ResMut<RenderSubPassTerrainGrid>) {
        // Create the 2d quad mesh
        let mesh: Handle<Mesh> = assets_server.add(Mesh {
            label: "terrain-grid-pass".to_string(),
            vertices: vec![
                Vertex { position: [-1.0, 1.0, 0.0], uv: [0.0, 1.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] },
                Vertex { position: [-1.0, -1.0, 0.0], uv: [0.0, 0.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] },
                Vertex { position: [1.0, -1.0, 0.0], uv: [1.0, 0.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] },
                Vertex { position: [1.0, 1.0, 0.0], uv: [1.0, 1.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] },
            ],
            indices: vec![0, 2, 1, 0, 3, 2],
            bbox: MeshBbox {
                min: Vec3::new(-1.0, -1.0, 0.0),
                max: Vec3::new(1.0, 1.0, 0.0),
            },
            use_ssbo: false,
        });
        pass.mesh = Some(mesh);
    }

    pub fn extract(
        pass_main: ExtractWorld<Res<RenderSubPassTerrainGrid>>,
        placement_ui: ExtractWorld<Res<PlacementUI>>,
        mut pass_render: ResMut<RenderSubPassTerrainGrid>,
    ) {
        pass_render.mesh = pass_main.mesh.clone();
        pass_render.render_grid = placement_ui.enabled;
    }
}
impl RenderSubPass for RenderSubPassTerrainGrid {
    type Params = (SRes<RenderAssets<GpuTerrainGridRenderPipeline>>, SRes<RenderSubPassTerrainGrid>, SRes<CameraFeatureRender>, SRes<TerrainGridBuffer>);

    fn describe(
        (pipeline, subpass, camera_feature, grid_buffer): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        if !subpass.render_grid {
            return RenderSubPassDesc::default();
        }
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(Some(pipeline.iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::Mesh(subpass.mesh.as_ref().map(|m| m.id())),
            SubPassCommand::BindGroup(0, camera_feature.bind_group.clone()),
            SubPassCommand::BindGroup(1, grid_buffer.bind_group.clone()),
            SubPassCommand::DrawBatches(vec![DrawCommandsBatch {
                index_range: 0..6,
                instance_range: 0..CHUNK_COUNT * CHUNK_COUNT,
                ..Default::default()
            }])
        ])
    }

    fn label() -> &'static str { "terrain-grid-subpass" }
}
