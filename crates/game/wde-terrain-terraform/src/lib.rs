use wde_logger::prelude::*;
use wde_gizmos::prelude::*;
use wde_renderer::prelude::*;
use wde_physics::prelude::*;
use wde_camera::prelude::*;
use wde_terrain::prelude::*;
use wde_egui::prelude::*;
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
            .init_resource::<TerraformingSettings>()
            .init_resource::<PaintingSettings>()
            .add_systems(Startup, spawn_dummy_cube)
            .add_systems(Startup, init)
            .add_systems(Update, mouse_picking)
            .add_systems(Update, (terrain_terraforming_gui, terrain_painting))
            .add_systems(Update, (apply_brush, apply_paint));
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

#[derive(Resource)]
pub struct TerraformingSettings {
    pub is_drawing: bool,
    pub world_position: Vec3,
    pub brush_size: f32,
    pub brush_strength: f32,
}
impl Default for TerraformingSettings {
    fn default() -> Self {
        Self {
            is_drawing: false,
            world_position: Vec3::ZERO,
            brush_size: 15.0,
            brush_strength: 0.01,
        }
    }
}

#[derive(Resource)]
pub struct PaintingSettings {
    pub is_drawing: bool,
    pub world_position: Vec3,
    pub brush_size: f32,
    pub brush_strength: f32,
    pub paint_color: [u8; 3],
}
impl Default for PaintingSettings {
    fn default() -> Self {
        Self {
            is_drawing: false,
            world_position: Vec3::ZERO,
            brush_size: 15.0,
            brush_strength: 0.1,
            paint_color: [255, 0, 0],
        }
    }
}

fn terrain_terraforming_gui(ctx: Res<EguiContext>, mut terraforming_settings: ResMut<TerraformingSettings>) {
    // egui::Window::new("Terrain Terraforming").show(&ctx.0, |ui| {
    //     ui.checkbox(&mut terraforming_settings.is_drawing, "Enable Terraforming");
    //     ui.add(egui::Slider::new(&mut terraforming_settings.brush_size, 0.1..=100.0).text("Brush Size"));
    //     ui.add(egui::Slider::new(&mut terraforming_settings.brush_strength, 0.0001..=0.01).text("Brush Strength"));
    // });
}

fn terrain_painting(ctx: Res<EguiContext>, mut terraforming_settings: ResMut<PaintingSettings>) {
    egui::Window::new("Terrain Painting").show(&ctx.0, |ui| {
        ui.checkbox(&mut terraforming_settings.is_drawing, "Enable Painting");
        ui.add(egui::Slider::new(&mut terraforming_settings.brush_size, 0.1..=100.0).text("Brush Size"));
        ui.add(egui::Slider::new(&mut terraforming_settings.brush_strength, 0.1..=1.0).text("Brush Strength"));
        ui.color_edit_button_srgb(&mut terraforming_settings.paint_color);
    });
}

fn mouse_picking(
    mut command: Commands,
    phworld: Res<PhysicsWorld>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Transform, &CameraView), With<Camera>>,
    mut terraforming_settings: ResMut<TerraformingSettings>,
    mut painting_settings: ResMut<PaintingSettings>,
    dummy_entity: Res<DummyEntityHolder>,
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
    if let Some((_, toi)) = phworld.cast_ray(&ray, &RayCastConfig::default()) {
        let hit_point = ray.point_at(toi);
        terraforming_settings.world_position = hit_point;
        painting_settings.world_position = hit_point;
        command.entity(dummy_entity.0.unwrap()).insert(Transform::from_translation(hit_point));
    }
}

fn apply_brush(
    terraforming_settings: Res<TerraformingSettings>,
    mut terrain: Query<&mut Terrain>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    // Check if terraforming is enabled
    if !terraforming_settings.is_drawing {
        return;
    }

    // Get the terrain
    let Ok(mut terrain) = terrain.single_mut() else { return };

    // Check if the left mouse button is pressed
    if !mouse_input.pressed(MouseButton::Left) {
        return;
    }

    // Get tile data for the current world position
    let pos = match terrain.get_tile_idx_for_world_pos(terraforming_settings.world_position) {
        Some(pos) => pos,
        None => {
            warn!("World position ({}, {}, {}) is out of terrain bounds", terraforming_settings.world_position.x, terraforming_settings.world_position.y, terraforming_settings.world_position.z);
            return;
        }
    };
    let mut data = match terrain.get_tile_data_for_chunk(pos, 0, 0) {
        Some(data) => data.clone(),
        None => {
            warn!("No terrain data found for tile at position ({}, {})", pos.x, pos.y);
            return;
        }
    };

    // Apply the brush effect in a small radius around the world position
    let in_tile_pos = Vec2::new(
        (terraforming_settings.world_position.x - (pos.x as f32 * TILE_SIZE[0])) / TILE_SIZE[0] + 0.5,
        (terraforming_settings.world_position.z - (pos.y as f32 * TILE_SIZE[2])) / TILE_SIZE[2] + 0.5,
    );
    let radius = terraforming_settings.brush_size / TILE_SIZE[0]; // Brush size in tile space
    let ss = (data.len() / 4) as f32; // Subdivision size of the tile (assuming 4 channels per pixel)
    let ss = ss.sqrt(); // Square root to get the side length of the square grid
    for x in 0..ss as usize {
        for y in 0..ss as usize {
            let tile_pos = Vec2::new(x as f32 / ss, y as f32 / ss);
            let distance = tile_pos.distance(in_tile_pos);
            if distance < radius {
                let strength = (1.0 - (distance / radius)) * terraforming_settings.brush_strength;
                let idx = (y * ss as usize + x) * 4; // RGBA channels
                data[idx] += strength; // Increase height by brush strength, you can modify this to create different effects
            }
        }
    }

    // Write the modified data back to the terrain
    terrain.set_tile_data_for_chunk(pos, 0, 0, data);
}

fn apply_paint(
    painting_settings: Res<PaintingSettings>,
    mut terrain: Query<&mut Terrain>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    // Check if painting is enabled
    if !painting_settings.is_drawing {
        return;
    }

    // Get the terrain
    let Ok(mut terrain) = terrain.single_mut() else { return };

    // Check if the left mouse button is pressed
    if !mouse_input.pressed(MouseButton::Left) {
        return;
    }

    // Get tile data for the current world position
    let pos = match terrain.get_tile_idx_for_world_pos(painting_settings.world_position) {
        Some(pos) => pos,
        None => {
            warn!("World position ({}, {}, {}) is out of terrain bounds", painting_settings.world_position.x, painting_settings.world_position.y, painting_settings.world_position.z);
            return;
        }
    };
    let mut data = match terrain.get_tile_data_for_chunk(pos, 1, 0) {
        Some(data) => data.clone(),
        None => {
            warn!("No terrain data found for tile at position ({}, {})", pos.x, pos.y);
            return;
        }
    };

    // Apply the brush effect in a small radius around the world position
    let in_tile_pos = Vec2::new(
        (painting_settings.world_position.x - (pos.x as f32 * TILE_SIZE[0])) / TILE_SIZE[0] + 0.5,
        (painting_settings.world_position.z - (pos.y as f32 * TILE_SIZE[2])) / TILE_SIZE[2] + 0.5,
    );
    let radius = painting_settings.brush_size / TILE_SIZE[0]; // Brush size in tile space
    let ss = (data.len() as f32 / 4.0).sqrt(); // Subdivision size of the tile (divided by 4 for RGBA)
    for x in 0..ss as usize {
        for y in 0..ss as usize {
            let tile_pos = Vec2::new(x as f32 / ss, y as f32 / ss);
            let distance = tile_pos.distance(in_tile_pos);
            if distance < radius {
                let strength = (1.0 - (distance / radius)) * painting_settings.brush_strength;
                let idx = (y * ss as usize + x) * 4; // RGBA channels
                data[idx] = painting_settings.paint_color[0] as f32 / 255.0 * strength + data[idx] * (1.0 - strength);
                data[idx + 1] = painting_settings.paint_color[1] as f32 / 255.0 * strength + data[idx + 1] * (1.0 - strength);
                data[idx + 2] = painting_settings.paint_color[2] as f32 / 255.0 * strength + data[idx + 2] * (1.0 - strength);
            }
        }
    }

    // Write the modified data back to the terrain
    println!("Painting tile at position ({}, {}) with color ({}, {}, {}) and strength {}", pos.x, pos.y, painting_settings.paint_color[0], painting_settings.paint_color[1], painting_settings.paint_color[2], painting_settings.brush_strength);
    terrain.set_tile_data_for_chunk(pos, 1, 0, data);

}
