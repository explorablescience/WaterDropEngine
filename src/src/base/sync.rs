use std::collections::HashMap;

use wde::prelude::*;

use bevy::prelude::*;

use crate::base::{Base, InBase};

/// Spawns a [`Base`] for each group of linked buildings with a single control room, and puts its
/// buildings and their modules [`InBase`]. Other groups have no base: their buildings do not work.
pub(super) fn update_bases(
    mut commands: Commands,
    graph: Res<BuildingGraph>,
    added_modules: Query<(), Added<InteriorModule>>,
    placed: Query<(
        Entity,
        &GridEntity,
        Option<&InteriorModule>,
        Option<&InBase>
    )>,
    bases: Query<(Entity, &Base)>,
    mut spawned: Local<usize>
) {
    if !graph.is_changed() && added_modules.is_empty() {
        return;
    }

    // Control room of each building in a base
    let mut room_of = HashMap::new();
    for group in graph.groups() {
        let mut rooms = group.iter().filter(|e| {
            placed
                .get(**e)
                .is_ok_and(|(_, b, _, _)| b.entry().role.as_deref() == Some("control_room"))
        });
        if let (Some(room), None) = (rooms.next(), rooms.next()) {
            room_of.extend(group.iter().map(|e| (*e, *room)));
        }
    }

    // Bases are kept while their control room still defines one
    let mut base_of_room = HashMap::new();
    for (base, Base { control_room, .. }) in &bases {
        match room_of.get(control_room) == Some(control_room) {
            true => {
                base_of_room.insert(*control_room, base);
            }
            false => commands.entity(base).despawn()
        }
    }
    for room in room_of.values() {
        base_of_room.entry(*room).or_insert_with(|| {
            let name = base_name(*spawned);
            *spawned += 1;
            commands
                .spawn((
                    Name::new(name.clone()),
                    Base {
                        name,
                        control_room: *room
                    }
                ))
                .id()
        });
    }

    // Modules work in the base of their host
    for (entity, _, module, current) in &placed {
        let building = module.map_or(entity, |m| m.host);
        let base = room_of.get(&building).map(|room| base_of_room[room]);
        if base == current.map(|b| b.0) {
            continue;
        }
        match base {
            Some(base) => {
                commands.entity(entity).insert(InBase(base));
            }
            None => {
                commands.entity(entity).try_remove::<InBase>();
            }
        }
    }
}

const BASE_NAMES: [&str; 12] = [
    "Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta", "Theta", "Iota", "Kappa",
    "Lambda", "Omega"
];

/// `Base Alpha`, ..., `Base Omega`, then `Base Alpha 2`, ...
fn base_name(index: usize) -> String {
    let name = BASE_NAMES[index % BASE_NAMES.len()];
    match index / BASE_NAMES.len() {
        0 => format!("Base {name}"),
        n => format!("Base {name} {}", n + 1)
    }
}
