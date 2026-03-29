use wde_physics::prelude::*;
use wde_camera::prelude::*;
use bevy::{prelude::*, window::PrimaryWindow};
use wde_renderer::prelude::ExtractWorld;

/// Position of the cursor in world space, updated every frame by casting a ray from the camera to the terrain and finding the intersection point.
/// This resource is available globally in both the main and render worlds.
#[derive(Resource, Default)]
pub struct TerrainCursorPos {
    pub world_pos: Vec3
}
impl TerrainCursorPos {
    pub(crate) fn update(
        phworld: Res<PhysicsWorld>,
        window: Single<&Window, With<PrimaryWindow>>,
        camera_query: Query<(&Transform, &CameraView), With<Camera>>,
        mut terrain_cursor_pos: ResMut<TerrainCursorPos>
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
        if let Some((_entity, toi)) = phworld.cast_ray(&ray, &RayCastConfig::default()) {
            let hit_point = ray.point_at(toi);
            terrain_cursor_pos.world_pos = hit_point;
        }
    }

    pub(crate) fn extract(
        main_cursor_pos: ExtractWorld<Res<TerrainCursorPos>>,
        mut render_cursor_pos: ResMut<TerrainCursorPos>
    ) {
        render_cursor_pos.world_pos = main_cursor_pos.world_pos;
    }
}
