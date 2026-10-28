use bevy::prelude::*;
use wde::prelude::*;

use crate::colonist::{ColonistMarker, ENTITIES_COLLIDER_GROUP, MSpawnColonist, animate::WalkTracker};

/// The parent entity for all colonists
#[derive(Resource, Reflect)]
#[reflect(Resource)]
struct ColonistParent(pub Entity);

/// The models for colonists
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct ColonistModels {
    pub suit: Handle<GltfAsset>,
    pub indoor: Handle<GltfAsset>,
}

/// The queue of colonists to spawn
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
struct ColonistSpawnQueue(Vec<MSpawnColonist>);


pub struct SpawnColonistPlugin;
impl Plugin for SpawnColonistPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_message::<MSpawnColonist>()
            .init_resource::<ColonistSpawnQueue>()
            .register_type::<ColonistParent>()
            .register_type::<ColonistModels>()
            .register_type::<ColonistSpawnQueue>()
            .add_systems(Startup, init)
            .add_systems(Update, (spawn_colonists_to_queue, spawn_colonists, swap_colonist_model));
    }
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Create a parent entity for all colonists
    let colonist_parent = commands
        .spawn((Name::new("Colonists"), Transform::default()))
        .id();
    commands.insert_resource(ColonistParent(colonist_parent));

    // Load the colonist models
    commands.insert_resource(ColonistModels {
        suit: asset_server.load("models/entities/human/human.gltf"),
        indoor: asset_server.load("models/entities/human/human_indoor.gltf")
    });
}

/// Read the colonist spawn messages and add them to the spawn queue
fn spawn_colonists_to_queue(
    mut messages: MessageReader<MSpawnColonist>,
    mut queue: ResMut<ColonistSpawnQueue>,
) {
    for message in messages.read() {
        queue.0.push(message.clone());
    }
}

/// The margin to apply to the colonist's bounding box when creating the collider for picking
const COLONIST_PICK_MARGIN: f32 = 1.6;
/// Spawn colonists from the spawn queue
fn spawn_colonists(
    mut commands: Commands,
    colonist_parent: Res<ColonistParent>,
    colonist_models: Res<ColonistModels>,
    gltf_assets: Res<Assets<GltfAsset>>,
    mut colonist_spawn_queue: ResMut<ColonistSpawnQueue>,
) {
    // Get the bbox from one colonist model
    let Some(suit) = gltf_assets.get(&colonist_models.suit) else {
        return; // If the model is not loaded yet, return early
    };
    let bbox_extent = suit.bbox.max - suit.bbox.min;
    let bbox_offset = suit.bbox.min + bbox_extent / 2.0;
    let pick_extent = bbox_extent * Vec3::new(COLONIST_PICK_MARGIN, 1.0, COLONIST_PICK_MARGIN);

    // Spawn colonists from the queue
    for message in colonist_spawn_queue.0.drain(..) {
        // Check if the colonist is indoors based on the y position
        let is_indoor = message.position.y > 0.0;

        // Spawn the colonist entity
        let colonist = commands.spawn((
            Name::new(message.name.clone()),
            Transform::from_translation(message.position),
            Indoors(is_indoor),
            AnimationPlayer::default(),
            NavAgent::default(),
            ColonistMarker,
            WalkTracker::default(),
            ChildOf(colonist_parent.0),
        )).id();

        // The model is spawned by `swap_colonist_model`, as `Indoors` is just added

        // Spawn colonist collider
        commands.spawn((
            Name::new(format!("{} Collider", message.name)),
            Transform::from_translation(bbox_offset),
            Collider::from(BoxCollider::new(pick_extent).with_group(ENTITIES_COLLIDER_GROUP)),
            ChildOf(colonist)
        ));
        debug!("Spawned colonist: {} at {:?}", message.name, message.position);
    }
}


/// Swaps colonists between their suit and indoor models
fn swap_colonist_model(
    mut commands: Commands,
    models: Res<ColonistModels>,
    changed: Query<(Entity, &Indoors, Option<&Children>), (Changed<Indoors>, With<ColonistMarker>)>,
    model_parts: Query<(), Or<(With<Mesh3d>, With<AnimationBone>)>>,
    mut gltf_spawn_queue: ResMut<GltfSpawnQueue>
) {
    for (entity, indoors, children) in &changed {
        for child in children.into_iter().flatten() {
            if model_parts.contains(*child) {
                commands.entity(*child).despawn();
            }
        }
        let model = if indoors.0 { &models.indoor } else { &models.suit };
        GltfLoader::spawn(&mut gltf_spawn_queue, model.clone(), entity);
    }
}
