use bevy::{asset::LoadState, prelude::*};
use wde_gltf::prelude::*;
use wde_logger::prelude::*;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

use crate::prelude::GridEntity;

/// Key toggling [BuildingCutaway].
const CUTAWAY_TOGGLE_KEY: KeyCode = KeyCode::KeyV;

/// When enabled, placed buildings show their cutaway model (open top) instead of the full one.
/// Entries without a cutaway model keep showing the full one.
#[derive(Resource, Default)]
pub struct BuildingCutaway(pub bool);

/// Variant currently spawned under a placed building: `Some(true)` for the cutaway model,
/// `None` until its meshes are spawned.
#[derive(Component, Default)]
pub(crate) struct BuildingVariant(Option<bool>);

/// Mesh child of a placed building, respawned when the variant changes.
#[derive(Component)]
pub(crate) struct BuildingMesh;

pub(crate) fn toggle_cutaway(
    keys: Res<ButtonInput<KeyCode>>,
    mut cutaway: ResMut<BuildingCutaway>
) {
    if keys.just_pressed(CUTAWAY_TOGGLE_KEY) {
        cutaway.0 = !cutaway.0;
        debug!("Building cutaway view: {}.", cutaway.0);
    }
}

pub(crate) fn sync_building_variants(
    mut commands: Commands,
    cutaway: Res<BuildingCutaway>,
    asset_server: Res<AssetServer>,
    gltf_assets: Res<Assets<GltfAsset>>,
    mut buildings: Query<(Entity, &GridEntity, &mut BuildingVariant, Option<&Children>)>,
    meshes: Query<(), With<BuildingMesh>>
) {
    for (entity, grid_entity, mut variant, children) in &mut buildings {
        let entry = grid_entity.entry();
        // A cutaway model that failed to load falls back to the full one
        let cut = cutaway.0
            && entry.asset_cut.as_ref().is_some_and(|h| {
                !matches!(asset_server.get_load_state(h), Some(LoadState::Failed(_)))
            });
        if variant.0 == Some(cut) {
            continue;
        }
        let handle = match (&entry.asset_cut, cut) {
            (Some(asset_cut), true) => asset_cut,
            _ => &entry.asset
        };
        let Some(model) = gltf_assets.get(handle) else {
            continue; // Retry once loaded
        };

        if let Some(children) = children {
            for child in children.iter() {
                if meshes.contains(child) {
                    commands.entity(child).despawn();
                }
            }
        }
        for (mesh, material) in model.models.iter() {
            commands.spawn((
                Name::new(format!("{} Mesh", entry.label)),
                Transform::default(),
                grid_entity.clone(),
                Mesh3d(mesh.clone()),
                PbrMaterial3d(material.clone()),
                CastShadow,
                BuildingMesh,
                ChildOf(entity)
            ));
        }
        variant.0 = Some(cut);
    }
}
