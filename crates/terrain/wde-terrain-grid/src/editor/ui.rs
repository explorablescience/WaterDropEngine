use wde_editor::prelude::*;
use wde_pbr::prelude::*;
use wde_physics::prelude::*;
use wde_camera::prelude::*;
use wde_gltf::prelude::*;
use bevy::{prelude::*, window::PrimaryWindow};

use crate::prelude::{Grid, GridEntity, GridRotation};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PlacementTool {
    #[default]
    Place
}

#[derive(Resource)]
pub struct PlacementUI {
    pub enabled: bool,
    pub view_grid: bool,
    pub selected_tools: PlacementTool,

    pub placement_show_entity: bool,
    pub placement_entity: Option<Entity>,
    pub placement_entity_has_model: bool,
    pub placement_extent: UVec2,
    pub placement_model: Option<PbrModel>,
}
impl Default for PlacementUI {
    fn default() -> Self {
        PlacementUI {
            enabled: false,
            view_grid: false,
            selected_tools: PlacementTool::default(),

            placement_show_entity: false,
            placement_entity: None,
            placement_entity_has_model: false,
            placement_extent: UVec2::new(2, 2),
            placement_model: None
        }
    }
}

pub fn init_placement(mut commands: Commands, mut placement_ui: ResMut<PlacementUI>, asset_server: Res<AssetServer>) {
    // For now, load a model for placement preview
    let gltf_asset = GltfLoader::load("tests/models/houses/house_demo1.gltf", &asset_server).unwrap();
    placement_ui.placement_model = Some(PbrModel(gltf_asset.models.clone()));
    placement_ui.placement_entity_has_model = true;

    // Create an empty entity for placement preview
    let entity = commands.spawn((
        Name::new("Terrain Placement Preview"),
        Transform::IDENTITY
    )).id();
    placement_ui.placement_entity = Some(entity);
}

pub fn init_ui(mut ui_menu: ResMut<UIMenu>) {
    ui_menu.push("Terrain/Placement");
}

pub fn placement_system_ui(ctx: Res<UIContext>, ui_menu: Res<UIMenu>, mut placement_ui: ResMut<PlacementUI>) {
    if !ui_menu.is_clicked("Terrain/Placement") {
        return;
    }

    UIWindow::new("Placement Debug").show(&ctx.0, |ui| {
        ui.checkbox(&mut placement_ui.enabled, "Enable Placement Tool");
        ui.checkbox(&mut placement_ui.view_grid, "View Grid");
        ui.separator();
        ui.label("Selected Tool:");
        ui.selectable_value(&mut placement_ui.selected_tools, PlacementTool::Place, "Place");
        ui.separator();
        ui.checkbox(&mut placement_ui.placement_show_entity, "Show Placement Preview");
        ui.label("Placement Extent:");
        ui.horizontal(|ui| {
            ui.add(DragValue::new(&mut placement_ui.placement_extent.x).range(1..=10).prefix("x: "));
            ui.add(DragValue::new(&mut placement_ui.placement_extent.y).range(1..=10).prefix("y: "));
        });
    });
}

#[allow(clippy::too_many_arguments)]
pub fn handle_placement_tool(
    mut commands: Commands,
    mut placement_ui: ResMut<PlacementUI>,
    phworld: Res<PhysicsWorld>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Transform, &CameraView), With<Camera>>,
    mut grid: ResMut<Grid>,
    mut local_rot: Local<GridRotation>,
    mouse_input: Res<ButtonInput<MouseButton>>
) {
    
    // If placement entity should not be shown, remove the model from it if it exists and return early
    if !placement_ui.placement_show_entity && let Some(entity) = placement_ui.placement_entity && placement_ui.placement_entity_has_model {
        commands.entity(entity).remove::<PbrModel>();
        grid.remove_entity(Entity::PLACEHOLDER);
        placement_ui.placement_entity_has_model = false;
    }

    // Check if placement tool is enabled in the UI or if the placement entity is set
    if !placement_ui.enabled || placement_ui.selected_tools != PlacementTool::Place || placement_ui.placement_entity.is_none() {
        // Remove the model from the preview entity if it exists
        if let Some(entity) = placement_ui.placement_entity && placement_ui.placement_entity_has_model {
            commands.entity(entity).remove::<PbrModel>();
            grid.remove_entity(Entity::PLACEHOLDER);
            placement_ui.placement_entity_has_model = false;
        }
        return;
    }

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
        grid.remove_entity(Entity::PLACEHOLDER);
        
        // Update the grid with the new position of the entity
        let grid_entity = GridEntity::new(hit_point, placement_ui.placement_extent, *local_rot);
        commands.entity(placement_ui.placement_entity.unwrap()).insert(
            Transform::from_rotation(Quat::from_rotation_y(local_rot.rotation()))
                .with_translation(Vec3::new(grid_entity.center().x, 0.0, grid_entity.center().y))
        );
        let occupied_tiles = grid_entity.footprint();
        for (chunk_pos, local_pos) in occupied_tiles {
            grid.set_entity_at(*chunk_pos, *local_pos, Entity::PLACEHOLDER);
        }

        // Set the model on the preview entity if not already set
        if !placement_ui.placement_entity_has_model {
            commands.entity(placement_ui.placement_entity.unwrap()).insert(placement_ui.placement_model.clone().unwrap());
            placement_ui.placement_entity_has_model = true;
        }
    }
}
