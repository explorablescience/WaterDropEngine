use bevy::ecs::world::World;

use crate::serializers::scene::serialize_scene;

pub(crate) fn serialize_world(world: &mut World) {
    let scene = serialize_scene(world);
    
    println!("Serialized Scene:\n{}", scene);
}