use bevy::prelude::*;

use crate::camera::{ActiveCamera, Camera};

/// Marker for the camera visibility is computed from (PBR culling, terrain chunks). Defaults to the
/// [`ActiveCamera`].
#[derive(Component, Default, Clone, Debug, Reflect)]
#[reflect(Component)]
#[require(Camera)]
pub struct CullingCamera;

/// Marker for the camera moved by the camera controllers. Defaults to the [`ActiveCamera`].
#[derive(Component, Default, Clone, Debug, Reflect)]
#[reflect(Component)]
#[require(Camera)]
pub struct ControlledCamera;

/// What a camera is used for. Each role is held by a single camera.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraRole {
    /// Rendered on screen: [`ActiveCamera`].
    Rendered,
    /// [`CullingCamera`].
    Culling,
    /// [`ControlledCamera`].
    Controlled
}
impl CameraRole {
    pub const ALL: [Self; 3] = [Self::Rendered, Self::Culling, Self::Controlled];

    pub fn label(self) -> &'static str {
        match self {
            Self::Rendered => "Rendered",
            Self::Culling => "Culling",
            Self::Controlled => "Controlled"
        }
    }
}

/// Gives `role` to `camera`, taking it from the camera holding it:
/// ```rust
/// commands.queue(SetCameraRole { role: CameraRole::Culling, camera });
/// ```
pub struct SetCameraRole {
    pub role: CameraRole,
    pub camera: Entity
}
impl Command for SetCameraRole {
    fn apply(self, world: &mut World) {
        match self.role {
            CameraRole::Rendered => move_marker::<ActiveCamera>(world, self.camera),
            CameraRole::Culling => move_marker::<CullingCamera>(world, self.camera),
            CameraRole::Controlled => move_marker::<ControlledCamera>(world, self.camera)
        }
    }
}

fn move_marker<M: Component + Default>(world: &mut World, camera: Entity) {
    let holders: Vec<Entity> = world
        .query_filtered::<Entity, With<M>>()
        .iter(world)
        .filter(|e| *e != camera)
        .collect();
    for holder in holders {
        world.entity_mut(holder).remove::<M>();
    }
    if let Ok(mut camera) = world.get_entity_mut(camera) {
        camera.insert(M::default());
    }
}

/// Gives the roles no camera holds to the [`ActiveCamera`].
pub(crate) fn assign_default_roles(
    mut commands: Commands,
    active: Query<Entity, With<ActiveCamera>>,
    culling: Query<(), With<CullingCamera>>,
    controlled: Query<(), With<ControlledCamera>>
) {
    let Ok(active) = active.single() else {
        return;
    };
    if culling.is_empty() {
        commands.entity(active).insert(CullingCamera);
    }
    if controlled.is_empty() {
        commands.entity(active).insert(ControlledCamera);
    }
}
