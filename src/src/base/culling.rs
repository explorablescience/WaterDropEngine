use bevy::{ecs::entity::EntityHashSet, prelude::*};
use wde::prelude::*;

use crate::base::{Base, BaseMembers, InBase};

/// Bounds of each base: the union of the full model bounds of its members.
pub(super) fn update_base_bounds(
    mut bases: Query<(&mut CullGroup, Option<&BaseMembers>), With<Base>>,
    changed_bases: Query<Entity, (With<Base>, Changed<BaseMembers>)>,
    moved_members: Query<&InBase, Changed<GlobalTransform>>,
    members: Query<(&GridEntity, &GlobalTransform)>,
    models: Res<Assets<GltfAsset>>
) {
    let mut dirty: EntityHashSet = changed_bases.iter().collect();
    dirty.extend(moved_members.iter().map(|b| b.0));

    for base in dirty {
        let Ok((mut group, base_members)) = bases.get_mut(base) else {
            continue;
        };
        group.0 = members
            .iter_many(base_members.into_iter().flat_map(|m| m.iter()))
            .filter_map(|(building, transform)| {
                let bbox = &models.get(&building.entry().asset)?.bbox;
                Some(Aabb::new(bbox.min, bbox.max).transformed(transform))
            })
            .fold(Aabb::EMPTY, Aabb::union);
    }
}
