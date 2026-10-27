mod spawn;
mod animate;

use wde::prelude::*;

use bevy::prelude::*;

/// Collider group for colonists and other entities that can interact with the terrain.
pub const ENTITIES_COLLIDER_GROUP: ColliderGroup = ColliderGroup::GROUP_4;

/// A marker that indicates a colonist entity
#[derive(Component)]
pub struct ColonistMarker;

/// A message to spawn a colonist in the world.
#[derive(Message, Clone)]
pub struct MSpawnColonist {
    pub name: String,
    pub position: Vec3,
}

pub struct ColonistPlugin;
impl Plugin for ColonistPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_message::<MSpawnColonist>()
            .add_plugins((animate::AnimateColonistPlugin, spawn::SpawnColonistPlugin));
    }
}
