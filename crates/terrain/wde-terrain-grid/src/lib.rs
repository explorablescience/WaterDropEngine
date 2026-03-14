use wde_physics::prelude::*;
use wde_camera::prelude::*;
use wde_renderer::prelude::*;
use wde_gizmos::prelude::*;
use bevy::{prelude::*, window::PrimaryWindow};

use crate::{core::{CorePlugin, grid::Grid, grid_entity::{GridEntity, GridEntityRotation}}, render::RenderPlugin};

mod core;
mod render;

pub mod prelude {
    pub use super::TerrainGridPlugin;
    pub use super::core::grid::{Grid, GridLocalDir};
    pub use super::core::grid_entity::{GridEntity, GridEntityRotation};
}

pub struct TerrainGridPlugin;
impl Plugin for TerrainGridPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(CorePlugin)
            .add_plugins(RenderPlugin);

        app
            .init_resource::<Grid>()
            .init_resource::<GridTestEntity>()
            .add_systems(Startup, init)
            .add_systems(Update, show_footprint_at_mouse_pos);
    }
}

#[derive(Resource, Default)]
pub struct GridTestEntity(Option<Entity>);

fn init(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut grid: ResMut<Grid>,
    mut test_entity: ResMut<GridTestEntity>
) {
    // Add a dummy entity to test the grid system
    let footprint = GridEntity {
        center: Vec2::new(5.0, 5.0),
        size: Vec2::new(2.0, 2.0),
        rotation: GridEntityRotation::R0
    };
    let entity = commands.spawn((
        Transform::from_xyz(footprint.center.x, 0.5, footprint.center.y),
        Mesh(asset_server.add(CapsuleMesh::from("test_gizmo", CapsuleMeshConfig::default()))),
        GizmoMaterial(asset_server.add(GizmoMaterialAsset {
            color: [0.8, 0.2, 0.2, 1.0],
            ..Default::default()
        })),
        footprint.clone()
    )).id();
    test_entity.0 = Some(entity);

    // Set the entity in the grid based on its footprint
    // let occupied_tiles = footprint.get_occupied_tiles();
    // for (chunk_pos, local_pos) in occupied_tiles {
    //     grid.set_entity_at_chunk_local(chunk_pos, local_pos, entity);
    // }


    // // Add a dummy entity to test the grid system
    // let footprint = GridEntity {
    //     center: Vec2::new(10.0, 6.0),
    //     size: Vec2::new(2.0, 4.0),
    //     rotation: GridEntityRotation::R90
    // };
    // let entity = commands.spawn(footprint.clone()).id();
    // let occupied_tiles = footprint.get_occupied_tiles();
    // for (chunk_pos, local_pos) in occupied_tiles {
    //     grid.set_entity_at_chunk_local(chunk_pos, local_pos, entity);
    // }


    // // Add a dummy entity to test the grid system
    // let footprint = GridEntity {
    //     center: Vec2::new(6.0, 10.0),
    //     size: Vec2::new(2.0, 2.0),
    //     rotation: GridEntityRotation::R45
    // };
    // let entity = commands.spawn(footprint.clone()).id();
    // let occupied_tiles = footprint.get_occupied_tiles();
    // for (chunk_pos, local_pos) in occupied_tiles {
    //     grid.set_entity_at_chunk_local(chunk_pos, local_pos, entity);
    // }
}

fn show_footprint_at_mouse_pos(
    mut commands: Commands,
    phworld: Res<PhysicsWorld>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Transform, &CameraView), With<Camera>>,
    test_entity: ResMut<GridTestEntity>,
    mut grid: ResMut<Grid>,
) {
    // Get the test entity
    let entity = match test_entity.0 {
        Some(entity) => entity,
        None => return,
    };

    // Create the ray from ndc position
    let cursor_ndc_position = match window.cursor_position() {
        Some(pos) => pos / window.size(),
        None => return,
    };
    let (camera_transform, camera_view) = camera_query.single().map_err(|_| "No camera found").unwrap();
    let ray = Ray::from_ndc(cursor_ndc_position, window.size().x / window.size().y, camera_transform, camera_view);

    // Cast the ray in the physics world
    if let Some((_, toi)) = phworld.as_ref().cast_ray(&ray, &RayCastConfig::default()) {
        let hit_point = ray.point_at(toi);

        // Move the test entity to the hit point
        commands.entity(entity).insert(Transform::from_xyz(hit_point.x, 0.5, hit_point.z));

        // Update the grid with the new position of the entity
        // let footprint = GridEntity {
        //     center: Vec2::new(hit_point.x, hit_point.z),
        //     size: Vec2::new(2.0, 2.0),
        //     rotation: GridEntityRotation::R0
        // };
        let pos = Vec2::new(hit_point.x, hit_point.z);
        grid.clear_all();
        let (chunk_pos, local_pos) = Grid::pos_to_chunk_and_local(pos);
        grid.set_entity_at(chunk_pos, local_pos, entity);
        // let occupied_tiles = footprint.get_occupied_tiles();
        // for (chunk_pos, local_pos) in occupied_tiles {
        //     grid.set_entity_at_chunk_local(chunk_pos, local_pos, entity);
        // }
    }
}
