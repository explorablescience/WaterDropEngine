use bevy::prelude::*;

use crate::base::{Base, BaseMembers};

/// Electricity produced by a building or module working in a base.
#[derive(Component, Clone, Copy, Debug)]
pub struct ElectricityProducer(pub f32);

/// Electricity consumed by a building or module working in a base.
#[derive(Component, Clone, Copy, Debug)]
pub struct ElectricityConsumer(pub f32);

/// Electricity balance of a [`Base`], summed over its members.
#[derive(Component, Default, PartialEq, Debug)]
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
    producers: Query<&ElectricityProducer>,
    consumers: Query<&ElectricityConsumer>
) {
    for (mut electricity, members) in &mut bases {
        let members = members.map(|m| m.iter()).into_iter().flatten();
        let (mut production, mut consumption) = (0.0, 0.0);
        for member in members {
            production += producers.get(member).map_or(0.0, |p| p.0);
            consumption += consumers.get(member).map_or(0.0, |c| c.0);
        }
        electricity.set_if_neq(BaseElectricity {
            production,
            consumption
        });
    }
}
