use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use rapier3d::{parry::utils::hashmap::HashMap, prelude::*};
use std::marker::{Send, Sync};

pub struct PhysicsPlugin;
impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, init)
            .add_systems(Update, handle_collider_changes);
    }
}


trait ColliderShape {
    fn build(&self) -> rapier3d::prelude::Collider;
}

struct CuboidCollider {
    hx: f32,
    hy: f32,
    hz: f32,
}
impl ColliderShape for CuboidCollider {
    fn build(&self) -> rapier3d::prelude::Collider {
        rapier3d::prelude::ColliderBuilder::cuboid(self.hx, self.hy, self.hz).build()
    }
}

#[derive(Component)]
#[require(Transform)]
pub struct Collider {
    data: Arc<RwLock<Box<dyn ColliderShape + Send + Sync>>>,
}
impl Collider {
    /// Create a cuboid collider component.
    /// This will also generate a fixed rigid body for the collider, positioned at the collider's transform.
    /// 
    /// # Arguments
    /// * `hx` - Half extent along the x-axis.
    /// * `hy` - Half extent along the y-axis.
    /// * `hz` - Half extent along the z-axis.
    /// 
    /// # Returns
    /// A new `Collider` component representing a cuboid collider.
    pub fn cuboid(hx: f32, hy: f32, hz: f32) -> Self {
        Collider {
            data: Arc::new(RwLock::new(Box::new(CuboidCollider { hx, hy, hz }))),
        }
    }
}

#[derive(Default)]
pub struct RapierHandler {
    /// The set of colliders in the physics world.
    collider_set: RwLock<ColliderSet>,
    /// The set of rigid bodies in the physics world.
    rigid_body_set: RwLock<RigidBodySet>,

    /// The query pipeline for spatial queries. Note that it will automatically filled by all the colliders and rigid bodies.
    query_pipeline: QueryPipeline,

    /// The island manager for the physics world.
    island_manager: RwLock<IslandManager>,
    /// The impulse joint set for the physics world.
    impulse_joint_set: RwLock<ImpulseJointSet>,
    /// The multibody joint set for the physics world.
    multibody_joint_set: RwLock<MultibodyJointSet>,
}

#[derive(Resource, Default)]
pub struct PhysicsWorld {
    /// The handler for Rapier physics.
    pub rapier: RapierHandler,

    // Mapping from Bevy entities to Rapier collider handles.
    entity_to_collider: HashMap<Entity, ColliderHandle>,
    collider_to_entity: HashMap<ColliderHandle, Entity>,
    entity_to_rigid_body: HashMap<Entity, RigidBodyHandle>,
    rigid_body_to_entity: HashMap<RigidBodyHandle, Entity>,
}



fn init(mut commands: Commands) {
    // Initialize the physics world resource. By default it is empty.
    commands.init_resource::<PhysicsWorld>();
}

fn handle_collider_changes(
    mut world: ResMut<PhysicsWorld>,
    colliders: Query<(Entity, &Transform, &Collider)>,
    new_collider: Query<Entity, Added<Collider>>,
    updated_collider: Query<Entity, Changed<Collider>>,
    updated_transform: Query<Entity, Changed<Transform>>,
    mut removed_collider: RemovedComponents<Collider>,
) {
    // Add new colliders to the physics world
    let _span = debug_span!("handle_new_colliders").entered();
    for entity in new_collider.iter().chain(updated_collider.iter()) {
        // Check if the collider already exists, and skip if it does
        if world.entity_to_collider.contains_key(&entity) {
            continue;
        }

        // Get the collider component and transform
        let (entity, transform, collider) = match colliders.get(entity) {
            Ok(data) => data,
            Err(_) => continue,
        };
        // Build the Rapier collider and insert it into the collider set
        let col = collider.data.read().unwrap().build();
        let col_handle = world.rapier.collider_set.write().unwrap().insert(col);

        // Create a rigid body for the collider and insert it into the rigid body set
        let rb = RigidBodyBuilder::fixed()
            .translation(vector![
                transform.translation.x,
                transform.translation.y,
                transform.translation.z
            ])
            .build();
        let rb_handle = world.rapier.rigid_body_set.write().unwrap().insert(rb);

        // Associate the collider with its rigid body and store the mapping
        world
            .rapier.collider_set.write().unwrap()
            .set_parent(col_handle, Some(rb_handle), &mut world.rapier.rigid_body_set.write().unwrap());
        world.entity_to_collider.insert(entity, col_handle);
        world.collider_to_entity.insert(col_handle, entity);
        world.entity_to_rigid_body.insert(entity, rb_handle);
        world.rigid_body_to_entity.insert(rb_handle, entity);
        debug!("Added collider and rigidbody for entity {:?} with collider {:?} and rigidbody {:?} handles", entity, col_handle, rb_handle);
    }
    drop(_span);

    // Handle removed colliders
    let _span = debug_span!("handle_removed_colliders").entered();
    removed_collider.read().for_each(|entity| {
        if let Some(&col_handle) = world.entity_to_collider.get(&entity) {
            // Remove the collider from the collider set
            world.rapier.rigid_body_set.write().unwrap().remove(
                world.entity_to_rigid_body[&entity],
                &mut world.rapier.island_manager.write().unwrap(),
                &mut world.rapier.collider_set.write().unwrap(),
                &mut world.rapier.impulse_joint_set.write().unwrap(),
                &mut world.rapier.multibody_joint_set.write().unwrap(),
                true,
            );

            // Remove the mappings
            world.entity_to_collider.remove(&entity);
            world.collider_to_entity.remove(&col_handle);
            debug!("Removed collider and rigidbody for entity {:?} with handle {:?}", entity, col_handle);
        }
    });
    drop(_span);

    // Handle updated colliders
    let _span = debug_span!("handle_updated_colliders").entered();
    for entity in updated_collider.iter() {
        if let Some(&col_handle) = world.entity_to_collider.get(&entity) {
            // Remove the old collider
            world.rapier.collider_set.write().unwrap().remove(
                col_handle,
                &mut world.rapier.island_manager.write().unwrap(),
                &mut world.rapier.rigid_body_set.write().unwrap(),
                true,
            );

            // Get the updated collider component
            let (entity, _, collider) = match colliders.get(entity) {
                Ok(data) => data,
                Err(_) => continue,
            };
            // Build the new Rapier collider and insert it into the collider set
            let col = collider.data.read().unwrap().build();
            let new_col_handle = world.rapier.collider_set.write().unwrap().insert(col);

            // Re-associate the new collider with its rigid body
            let rb_handle = world.entity_to_rigid_body[&entity];
            world
                .rapier.collider_set.write().unwrap()
                .set_parent(new_col_handle, Some(rb_handle), &mut world.rapier.rigid_body_set.write().unwrap());

            // Update the mappings
            world.entity_to_collider.insert(entity, new_col_handle);
            world.collider_to_entity.insert(new_col_handle, entity);
            debug!("Updated collider for entity {:?} with new handle {:?}", entity, new_col_handle);
        }
    }
    drop(_span);

    // Handle updated transforms
    let _span = trace_span!("handle_updated_transforms").entered();
    for entity in updated_transform.iter() {
        if let Some(&rb_handle) = world.entity_to_rigid_body.get(&entity) {
            // Get the updated transform
            let (_, transform, _) = match colliders.get(entity) {
                Ok(data) => data,
                Err(_) => continue,
            };
            // Update the rigid body's position
            if let Some(rigid_body) = world.rapier.rigid_body_set.write().unwrap().get_mut(rb_handle) {
                rigid_body.set_translation(
                    vector![
                        transform.translation.x,
                        transform.translation.y,
                        transform.translation.z
                    ],
                    true,
                );
                trace!("Updated transform for entity {:?} with rigidbody handle {:?}", entity, rb_handle);
            }
        }
    }
    drop(_span);
}







// fn init() {
    // let ground_body = RigidBodyBuilder::fixed().translation(vector![0.0, -1.0, 0.0]).build();
    // let ground_body_handle = rigid_body_set.insert(ground_body);
    // let ground_collider = ColliderBuilder::cuboid(50.0, 0.1, 50.0).build();
    // let h = collider_set.insert_with_parent(ground_collider, ground_body_handle, &mut rigid_body_set);
    // entity_to_collider.push((entity, h));

    // // Create the query pipeline
    // let mut query_pipeline = QueryPipeline::new();
    // query_pipeline.update(&collider_set);

    // // Create the physics world resource
    // commands.insert_resource(PhysicsWorld {
    //     collider_set,
    //     rigid_body_set,
    //     query_pipeline
    // });

    // if let Some((handle, intersection)) = query_pipeline.cast_ray_and_get_normal(
    //     &rigid_body_set, &collider_set, &ray, max_toi, solid, filter
    // ) {
    //     // This is similar to `QueryPipeline::cast_ray` illustrated above except
    //     // that it also returns the normal of the collider shape at the hit point.
    //     let hit_point = ray.point_at(intersection.time_of_impact);
    //     let hit_normal = intersection.normal;
    //     println!("Collider {:?} hit at point {} with normal {}", handle, hit_point, hit_normal);
    // }

    // query_pipeline.intersections_with_ray(&rigid_body_set, &collider_set, &ray, max_toi, solid, filter, |handle, intersection| {
    //     // Callback called on each collider hit by the ray.
    //     let hit_point = ray.point_at(intersection.time_of_impact);
    //     let hit_normal = intersection.normal;
    //     println!("Collider {:?} hit at point {} with normal {}", handle, hit_point, hit_normal);

    //     // Recover the entity associated to this collider, if any.
    //     let maybe_entity = entity_to_collider.iter().find_map(|(entity, h)| {
    //         if *h == handle {
    //             Some(*entity)
    //         } else {
    //             None
    //         }
    //     });
    //     if let Some(entity) = maybe_entity {
    //         println!("  -> associated entity: {:?}", entity);

    //         // // Move the cube up to the position of the hit point.
    //         // commands.entity(cube_entity).insert(Transform::from_translation(Vec3::new(
    //         //     hit_point.x, hit_point.y,
    //         //     hit_point.z,
    //         // )));
    //     }

    //     true // return `true` to continue the query.
    // });
// }

// fn cast_ray(mut commands: Commands, window: Single<&Window, With<PrimaryWindow>>, physics_world: Res<PhysicsWorld>, cube_entity_res: Res<CubeEntity>, camera_query: Query<(&Transform, &CameraView), With<Camera>>) {
//     let (collider_set, rigid_body_set, query_pipeline) = (
//         &physics_world.collider_set,
//         &physics_world.rigid_body_set,
//         &physics_world.query_pipeline,
//     );

//     // Get the cursor position
//     let Some(cursor_position) = window.cursor_position() else {
//         return;
//     };

//     // Get the camera transform
//     let (camera_transform, camera_view) = camera_query.single().map_err(|_| "No camera found").unwrap();

//     // Ray casting example. Convert the 2D cursor position to a 3D ray in world space using
//     let window_size = Vec2::new(window.width(), window.height());
//     let aspect_ratio = window_size.x / window_size.y;
//     let ndc_pos = (cursor_position / window_size) * 2.0 - Vec2::ONE;
//     let ndc_pos = Vec2::new(ndc_pos.x, -ndc_pos.y); // Invert Y for NDC
//     let dir = camera_view.ndc_to_world(ndc_pos, camera_transform, aspect_ratio);
//     let origin = camera_transform.translation;
//     let ray = Ray::new(point![origin.x, origin.y, origin.z], vector![dir.x, dir.y, dir.z]);

//     // Casting query examples:
//     if let Some((_, toi)) = query_pipeline.cast_ray(
//         rigid_body_set, collider_set, &ray, f32::MAX, true, QueryFilter::default()
//     ) {
//         let hit_point = ray.point_at(toi);

//         // Move the cube up to the position of the hit point.
//         commands.entity(cube_entity_res.0).insert(Transform::from_translation(Vec3::new(
//             hit_point.x, hit_point.y,
//             hit_point.z,
//         )));
//     }
// }
