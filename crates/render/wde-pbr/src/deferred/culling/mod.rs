//! Frustum culling of the PBR batches, on the CPU.
//!
//! Each frame, the instances of every batch are tested against the
//! [`CullingCamera`](wde_camera::prelude::CullingCamera) and each shadow
//! cascade. Their visible instances are written to their own region of the
//! [`PbrSsboInstanceToTransform`](crate::prelude::PbrSsboInstanceToTransform) buffer, and the
//! passes draw that range only.
//!
//! # Levels
//! 1. **Groups**: a [`CullGroup`] gives world-space bounds to a set of meshes, e.g. a village or a
//!    character. A group outside of a view hides all its meshes from it, and a group fully inside
//!    shows them all, both without testing them one by one.
//! 2. **Meshes**: the remaining meshes (in a group crossing the view edge, or in no group) are
//!    tested with the bounds of their mesh, moved by their transform.
//!
//! # Usage
//! ```rust
//! // Spawn the group, and keep its bounds up to date
//! let group = commands.spawn(CullGroup(Aabb::new(Vec3::splat(-20.0), Vec3::splat(20.0)))).id();
//! // Every mesh of the village is in the group
//! commands.spawn((Transform::default(), CullGroupMember(group)));
//! ```
//! Levels can be toggled with the [`CullingSettings`] resource, editable in the `PBR/Culling`
//! editor window along with the [`CullingStats`].
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::deferred::build_batches;

mod cull;
#[cfg(debug_assertions)]
mod editor;
mod groups;

pub use groups::{CullGroup, CullGroupMember, InCullGroup};

/// Culling views: the camera, then each shadow cascade.
pub const CULL_VIEW_COUNT: usize = 4;
pub(crate) const CAMERA_VIEW: usize = 0;
pub(crate) const fn shadow_view(cascade: u32) -> usize {
    1 + cascade as usize
}

#[derive(Resource, Clone, ExtractResource, Reflect)]
#[reflect(Resource)]
pub struct CullingSettings {
    /// Draw in each view only what lies in it.
    pub enabled: bool,
    /// Test the cull groups before their meshes.
    pub groups: bool,
    /// Test each mesh that its group does not decide on.
    pub meshes: bool,
    /// Cull the shadow casters of each cascade, not only the camera view.
    pub shadows: bool,
    /// Draw the bounds of the cull groups (with the `wde` debug gizmos).
    pub show_groups: bool
}
impl Default for CullingSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            groups: true,
            meshes: true,
            shadows: true,
            show_groups: false
        }
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct CullingStats {
    /// Batched instances.
    pub instances: usize,
    /// Instances drawn in each view.
    pub drawn: [usize; CULL_VIEW_COUNT],
    pub groups: usize,
    /// Groups outside of each view.
    pub groups_culled: [usize; CULL_VIEW_COUNT],
    /// Mesh bounds tested, over every view.
    pub mesh_tests: usize,
    /// CPU time of the culling, in microseconds, smoothed over frames.
    pub micros: f32
}

/// [`CullingStats`] of the last rendered frame, shared between the main and render worlds.
#[derive(Resource, Clone, Default)]
pub struct CullingStatsShared(pub Arc<Mutex<CullingStats>>);

pub(crate) struct CullingPlugin;
impl Plugin for CullingPlugin {
    fn build(&self, app: &mut App) {
        let stats = CullingStatsShared::default();
        app.init_resource::<CullingSettings>()
            .insert_resource(stats.clone())
            .register_type::<CullingSettings>()
            .register_type::<CullGroup>()
            .register_type::<CullGroupMember>()
            .register_type::<InCullGroup>()
            .add_plugins((
                ExtractResourcePlugin::<CullingSettings>::default(),
                ExtractComponentPlugin::<InCullGroup>::default()
            ))
            .add_systems(PostUpdate, groups::resolve_cull_groups);

        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .insert_resource(stats)
            .init_resource::<CullingSettings>()
            .init_resource::<cull::CameraFrustum>()
            .init_resource::<cull::ExtractedCullGroups>()
            .add_systems(
                Extract,
                (cull::extract_camera_frustum, cull::extract_cull_groups)
            )
            .add_systems(
                Render,
                (cull::update_world_bounds, cull::cull_instances)
                    .chain()
                    .after(build_batches)
                    .in_set(RenderSet::Prepare)
            );

        #[cfg(debug_assertions)]
        app.add_plugins(editor::CullingEditorPlugin);
    }
}
