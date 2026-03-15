use wde_physics::prelude::*;
use wde_camera::prelude::*;
use wde_gltf::prelude::*;
use wde_pbr::prelude::*;
use bevy::{prelude::*, window::PrimaryWindow};

use crate::{core::{CorePlugin, grid::Grid, grid_entity::{GridEntity, GridRotation}}, render::RenderPlugin};

mod core;
mod render;

pub mod prelude {
    pub use super::TerrainGridPlugin;
    pub use super::core::grid::{Grid, GridLocalDir, GridTilePos};
    pub use super::core::grid_entity::{GridEntity, GridRotation};
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
    mut test_entity: ResMut<GridTestEntity>
) {
    // Add a dummy entity to test the grid system
    let gltf_asset = GltfLoader::load("tests/models/houses/house_demo1.gltf", &asset_server).unwrap();
    let entity = commands.spawn((
        Transform::IDENTITY,
        PbrModel(gltf_asset.models.clone())
    )).id();
    test_entity.0 = Some(entity);
}

#[allow(clippy::too_many_arguments)]
fn show_footprint_at_mouse_pos(
    mut commands: Commands,
    phworld: Res<PhysicsWorld>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Transform, &CameraView), With<Camera>>,
    test_entity: ResMut<GridTestEntity>,
    mut grid: ResMut<Grid>,
    mut local_rot: Local<GridRotation>,
    mouse_input: Res<ButtonInput<MouseButton>>
) {
    // Get the test entity
    let entity = match test_entity.0 {
        Some(entity) => entity,
        None => return,
    };

    // Toggle rotation on right click
    if mouse_input.just_pressed(MouseButton::Middle) {
        *local_rot = match *local_rot {
            GridRotation::R0 => GridRotation::R90,
            GridRotation::R90 => GridRotation::R180,
            GridRotation::R180 => GridRotation::R270,
            GridRotation::R270 => GridRotation::R0
        };
    }

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
        let hit_point = Vec2::new(hit_point.x, hit_point.z);
        
        // Clear the grid of any registered entities
        grid.clear_all();

        // // Set the object at the closest tile to the hit point
        // let (pos_chunk, pos_local) = Grid::world_to_pos(hit_point);
        // grid.set_entity_at(pos_chunk, pos_local, Entity::PLACEHOLDER);
        // let pos = Grid::pos_to_world(pos_chunk, pos_local);
        // commands.entity(entity).insert(Transform::from_xyz(pos.x, 0.5, pos.y));
        
        
        // Update the grid with the new position of the entity
        let grid_entity = GridEntity::new(hit_point, UVec2::new(3, 2), *local_rot);
        // commands.entity(entity).insert(
        //     Transform::from_rotation(Quat::from_rotation_y(local_rot.rotation()))
        //         .with_translation(Vec3::new(grid_entity.center().x, 0.0, grid_entity.center().y))
        //         .with_scale(Vec3::new(1.0, 1.0, 1.0))
        // );
        let occupied_tiles = grid_entity.footprint();
        for (chunk_pos, local_pos) in occupied_tiles {
            grid.set_entity_at(*chunk_pos, *local_pos, entity);
        }
    }
}
