use bevy::{ecs::entity::EntityHashSet, prelude::*};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::prelude::PbrBatchesMarker;

/// World-space bounds of a group of meshes, culled together before being tested one by one.
/// Empty bounds disable the group test, so its meshes are only tested one by one.
#[derive(Component, Clone, Copy, Default, Debug, Reflect)]
#[reflect(Component)]
pub struct CullGroup(pub Aabb);

/// Puts the meshes of this entity and of its descendants in the [`CullGroup`] of the given
/// entity. The nearest one among the mesh and its ancestors wins.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct CullGroupMember(pub Entity);

/// Cull group of a mesh, resolved from the [`CullGroupMember`] of itself or of its ancestors.
#[derive(Component, Clone, Copy, PartialEq, Debug, Reflect, ExtractComponent)]
#[reflect(Component)]
#[extract_component_filter(Changed<Self>)]
pub struct InCullGroup(pub Entity);

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn resolve_cull_groups(
    mut commands: Commands,
    added: Query<
        Entity,
        (
            With<PbrBatchesMarker>,
            Or<(Added<PbrBatchesMarker>, Changed<ChildOf>)>
        )
    >,
    changed_members: Query<Entity, Changed<CullGroupMember>>,
    mut removed_members: RemovedComponents<CullGroupMember>,
    meshes: Query<Option<&InCullGroup>, With<PbrBatchesMarker>>,
    members: Query<&CullGroupMember>,
    children: Query<&Children>,
    parents: Query<&ChildOf>
) {
    let mut dirty: EntityHashSet = added.iter().collect();
    for root in changed_members.iter().chain(removed_members.read()) {
        dirty.insert(root);
        dirty.extend(children.iter_descendants(root));
    }

    for entity in dirty {
        let Ok(current) = meshes.get(entity) else {
            continue;
        };
        let group = std::iter::once(entity)
            .chain(parents.iter_ancestors(entity))
            .find_map(|e| members.get(e).ok())
            .map(|member| InCullGroup(member.0));
        match group {
            Some(group) if current != Some(&group) => {
                commands.entity(entity).try_insert(group);
            }
            None if current.is_some() => {
                commands.entity(entity).try_remove::<InCullGroup>();
            }
            _ => {}
        }
    }
}
