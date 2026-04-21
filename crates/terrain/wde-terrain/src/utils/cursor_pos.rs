use bevy::{prelude::*, window::PrimaryWindow};
use wde_camera::prelude::*;
use wde_physics::prelude::*;
use wde_renderer::prelude::*;

use crate::TERRAIN_COLLIDER_GROUP;

/// Resource to store the world position of the cursor on the terrain.
/// This is updated every frame by casting a ray from the camera through the cursor position and checking for intersections with the terrain colliders.
/// It is available both in the main and render world.
#[derive(Resource, Default, Clone, ExtractResource)]
pub struct TerrainCursorPos {
    pos: Option<Vec3>,
    last_non_none_pos: Vec3
}
impl TerrainCursorPos {
    pub(crate) fn update(
        phworld: Res<PhysicsWorld>,
        window: Single<&Window, With<PrimaryWindow>>,
        camera_query: Query<(&GlobalTransform, &CameraView), With<Camera>>,
        mut terrain_cursor_pos: ResMut<TerrainCursorPos>
    ) {
        // Get cursor position and camera data
        let (Some(cursor_pos), Ok((camera_transform, camera_view))) =
            (window.cursor_position(), camera_query.single())
        else {
            return;
        };

        // Create ray from camera
        let cursor_ndc = cursor_pos / window.size();
        let aspect_ratio = window.size().x / window.size().y;
        let ray = Ray::from_ndc(cursor_ndc, aspect_ratio, camera_transform, camera_view);

        // Cast the ray
        if let Some((_entity, toi)) =
            phworld.cast_ray(&ray, &RayCastConfig::with_filter(TERRAIN_COLLIDER_GROUP))
        {
            let hit_point = ray.point_at(toi);
            terrain_cursor_pos.pos = Some(hit_point);
            terrain_cursor_pos.last_non_none_pos = hit_point;
        } else {
            terrain_cursor_pos.pos = None;
        }
    }

    /// Returns the world position of the cursor on the terrain, if it is currently hovering over the terrain. Otherwise, it returns `None`.
    pub fn pos(&self) -> Option<Vec3> {
        self.pos
    }

    /// Returns the world position of the cursor on the terrain, if it is currently hovering over the terrain. Otherwise, it returns the last known position of the cursor on the terrain.
    pub fn pos_or_last(&self) -> Vec3 {
        self.pos.unwrap_or(self.last_non_none_pos)
    }
}
