use bevy::{ecs::entity::EntityHashSet, prelude::*};

use crate::base::{Base, BaseMembers, InBase};

/// Electricity produced by a building or module working in a base.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct ElectricityProducer(pub f32);

/// Electricity consumed by a building or module working in a base.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct ElectricityConsumer(pub f32);

/// Electricity balance of a [`Base`], summed over its members.
#[derive(Component, Default, PartialEq, Debug, Reflect)]
#[reflect(Component)]
pub struct BaseElectricity {
    pub production: f32,
    pub consumption: f32
}
impl BaseElectricity {
    pub fn balance(&self) -> f32 {
        self.production - self.consumption
    }

    /// Whether the base produces at least as much electricity as it consumes.
    pub fn is_sufficient(&self) -> bool {
        self.balance() >= 0.0
    }
}

pub(super) fn update_electricity(
    mut bases: Query<(&mut BaseElectricity, Option<&BaseMembers>), With<Base>>,
    changed_bases: Query<Entity, (With<Base>, Changed<BaseMembers>)>,
    changed_members: Query<
        &InBase,
        Or<(Changed<ElectricityProducer>, Changed<ElectricityConsumer>)>
    >,
    mut removed_members: RemovedComponents<BaseMembers>,
    mut removed_producers: RemovedComponents<ElectricityProducer>,
    mut removed_consumers: RemovedComponents<ElectricityConsumer>,
    members: Query<(
        Option<&InBase>,
        Option<&ElectricityProducer>,
        Option<&ElectricityConsumer>
    )>
) {
    // Despawned members and `InBase` changes mark the bases' `BaseMembers` as changed (or remove it).
    let mut dirty: EntityHashSet = changed_bases.iter().collect();
    dirty.extend(removed_members.read());
    dirty.extend(changed_members.iter().map(|b| b.0));
    dirty.extend(
        removed_producers
            .read()
            .chain(removed_consumers.read())
            .filter_map(|e| members.get(e).ok()?.0.map(|b| b.0))
    );

    for base in dirty {
        let Ok((mut electricity, base_members)) = bases.get_mut(base) else {
            continue;
        };
        let (mut production, mut consumption) = (0.0, 0.0);
        for (_, producer, consumer) in
            members.iter_many(base_members.into_iter().flat_map(|m| m.iter()))
        {
            production += producer.map_or(0.0, |p| p.0);
            consumption += consumer.map_or(0.0, |c| c.0);
        }
        electricity.set_if_neq(BaseElectricity {
            production,
            consumption
        });
    }
}
