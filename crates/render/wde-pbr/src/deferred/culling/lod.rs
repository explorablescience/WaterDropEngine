use bevy::prelude::*;
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

/// Selection of the levels of detail of the meshes (see [`Mesh::lods`]).
#[derive(Resource, Clone, ExtractResource, Reflect)]
#[reflect(Resource)]
pub struct LodSettings {
    /// Draw far meshes with their simplified levels of detail.
    pub enabled: bool,
    /// Largest error of the drawn level, in pixels on screen.
    pub max_pixel_error: f32,
    /// Draw every mesh with this level, or its coarsest one.
    pub force_level: Option<usize>,
    /// Color the meshes by drawn level: green, yellow, orange then red, and blue for the meshes
    /// without levels.
    pub show_levels: bool
}
impl LodSettings {
    /// Level `lod` of a mesh with `levels` levels, as given to the shaders: 0 when
    /// [`Self::show_levels`] is unset, `lod + 1`, or `MAX_MESH_LODS + 1` without levels.
    pub(crate) fn debug_lod(settings: Option<&Self>, lod: usize, levels: usize) -> u32 {
        match settings {
            Some(settings) if settings.show_levels && levels <= 1 => MAX_MESH_LODS as u32 + 1,
            Some(settings) if settings.show_levels => lod as u32 + 1,
            _ => 0
        }
    }
}
impl Default for LodSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_pixel_error: 1.0,
            force_level: None,
            show_levels: false
        }
    }
}

/// Culling camera, as seen by the level of detail selection.
#[derive(Clone, Copy)]
pub(crate) struct LodView {
    position: Vec3,
    forward: Vec3,
    /// Size on screen in pixels of one world unit at one unit of distance, on the view axis.
    pixels_per_unit: f32,
    /// Cosine of the angle between the view axis and the screen corners.
    min_cos: f32
}

#[derive(Resource, Default)]
pub(crate) struct LodCamera(pub Option<LodView>);

pub(super) fn extract_lod_camera(
    camera: ExtractWorld<Query<(&GlobalTransform, &CameraView), With<CullingCamera>>>,
    window: ExtractWorld<Query<&Window>>,
    mut lod_camera: ResMut<LodCamera>
) {
    lod_camera.0 = match (camera.single(), window.single()) {
        (Ok((transform, view)), Ok(window)) if window.height() > 0.0 => {
            let tan_half_fov = (view.fov.to_radians() * 0.5).tan();
            let aspect = window.width() / window.height();
            let tan_corner = tan_half_fov * (1.0 + aspect * aspect).sqrt();
            Some(LodView {
                position: transform.translation(),
                forward: transform.forward().as_vec3(),
                pixels_per_unit: window.resolution.physical_height() as f32 / (2.0 * tan_half_fov),
                min_cos: 1.0 / (1.0 + tan_corner * tan_corner).sqrt()
            })
        }
        _ => None
    };
}

/// Coarsest level of `lods` whose error on screen stays under the limit, for a mesh with
/// world-space `bounds` and largest `scale`.
pub(crate) fn select_lod(
    lods: &[GpuMeshLod],
    bounds: &Aabb,
    scale: f32,
    view: Option<&LodView>,
    settings: &LodSettings
) -> usize {
    let coarsest = lods.len().saturating_sub(1);
    if !settings.enabled || coarsest == 0 {
        return 0;
    }
    if let Some(level) = settings.force_level {
        return level.min(coarsest);
    }
    let Some(view) = view else {
        return 0;
    };
    // Nearest point of the bounds
    let to_bounds = view.position.clamp(bounds.min, bounds.max) - view.position;
    let distance = to_bounds.length();
    if distance <= 0.0 {
        return 0;
    }
    // The projection stretches the error by 1 / cos² of the angle to the view axis, at most up to
    // the screen corners (farther points are off screen, only drawn in the shadows)
    let cos = (to_bounds.dot(view.forward) / distance).clamp(view.min_cos, 1.0);
    let max_error =
        settings.max_pixel_error * distance * cos * cos / (view.pixels_per_unit * scale);
    lods.iter()
        .rposition(|lod| lod.error <= max_error)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LODS: [GpuMeshLod; 3] = [
        GpuMeshLod {
            first_index: 0,
            index_count: 300,
            error: 0.0
        },
        GpuMeshLod {
            first_index: 300,
            index_count: 150,
            error: 0.01
        },
        GpuMeshLod {
            first_index: 450,
            index_count: 75,
            error: 0.1
        }
    ];

    /// Level of a unit box at `distance` in front of the camera, `side` units off its axis.
    fn select_at(distance: f32, side: f32, scale: f32) -> usize {
        let view = LodView {
            position: Vec3::new(side, 0.0, distance + 1.0),
            forward: Vec3::NEG_Z,
            pixels_per_unit: 1000.0,
            min_cos: 0.5
        };
        let bounds = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        select_lod(&LODS, &bounds, scale, Some(&view), &LodSettings::default())
    }

    fn select(distance: f32, scale: f32) -> usize {
        select_at(distance, 0.0, scale)
    }

    #[test]
    fn coarser_with_distance() {
        assert_eq!(select(0.0, 1.0), 0);
        assert_eq!(select(5.0, 1.0), 0);
        assert_eq!(select(10.0, 1.0), 1);
        assert_eq!(select(99.0, 1.0), 1);
        assert_eq!(select(100.0, 1.0), 2);
    }

    #[test]
    fn finer_when_scaled_up() {
        assert_eq!(select(100.0, 2.0), 1);
    }

    #[test]
    fn finer_off_axis() {
        // Nearest point 100 units ahead and 100 to the side: 141 units away, at 45°
        assert_eq!(select_at(99.0, 101.0, 1.0), 1);
        // Far off screen, clamped to the corners
        assert_eq!(select_at(0.0, 401.0, 1.0), 2);
    }
}
