use wde_gizmos::prelude::*;
use wde_renderer::prelude::*;
use wde_physics::prelude::*;
use wde_camera::prelude::*;
use wde_terrain::prelude::*;
use bevy::{prelude::*, window::PrimaryWindow};

pub mod prelude {
    pub use super::TerrainTerraformPlugin;
}

pub struct TerrainTerraformPlugin;
impl Plugin for TerrainTerraformPlugin {
    fn build(&self, app: &mut App) {
        // Test system
        app
            .init_resource::<DummyEntityHolder>()
            .add_systems(Startup, spawn_dummy_cube)
            .add_systems(Startup, init)
            .add_systems(Update, mouse_picking);
    }
}


#[derive(Resource, Default)]
pub struct DummyEntityHolder(Option<Entity>);

fn spawn_dummy_cube(mut commands: Commands, asset_server: Res<AssetServer>, mut dummy_entity: ResMut<DummyEntityHolder>) {
    let entity =  commands.spawn((
        Transform::from_xyz(0.0, 1.0, 0.0),
        Mesh(asset_server.add(CapsuleMesh::from("character", CapsuleMeshConfig::default()))),
        GizmoMaterial(asset_server.add(GizmoMaterialAsset {
            label: "character".to_string(),
            color: [0.8, 0.4, 0.4, 1.0]
        })),
    ));
    dummy_entity.0 = Some(entity.id());
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Terrain::load("tests/terrain"),
        TerrainRenderer::new(&asset_server),
        TerrainPhysics::default()
    ));
}

fn mouse_picking(
    mut commands: Commands,
    phworld: Res<PhysicsWorld>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Transform, &CameraView), With<Camera>>,
    dummy_entity: Res<DummyEntityHolder>
) {
    // Get cursor position in NDC
    let Some(cursor_pos) = window.cursor_position() else { return };
    let cursor_ndc = cursor_pos / window.size();

    // Get camera data
    let Ok((camera_transform, camera_view)) = camera_query.single() else { return };
    let aspect_ratio = window.size().x / window.size().y;

    // Create ray from camera
    let ray = Ray::from_ndc(cursor_ndc, aspect_ratio, camera_transform, camera_view);

    // Cast the ray
    if let Some((entity, toi)) = phworld.cast_ray(&ray, &RayCastConfig::default()) {
        let hit_point = ray.point_at(toi);

        // Move the dummy entity to the hit point
        if let Some(dummy_entity) = dummy_entity.0 {
            commands.entity(dummy_entity).insert(Transform::from_translation(hit_point));
        }
    }
}
