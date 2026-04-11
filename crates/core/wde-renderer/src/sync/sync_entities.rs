use bevy::{ecs::entity::EntityEquivalent, prelude::*};


/// Component added on the Main world entities that are synced to the Render World in order to keep track of the corresponding render world entity.
/// It points to the corresponding entity in the render world.
#[derive(Component, Deref, Copy, Clone, Debug, Eq, Hash, PartialEq, Reflect)]
#[component(clone_behavior = Ignore)]
#[reflect(Component, Clone)]
pub struct RenderEntity(pub(crate) Entity);
impl RenderEntity {
    #[inline]
    pub fn id(&self) -> Entity {
        self.0
    }
}
impl From<Entity> for RenderEntity {
    fn from(entity: Entity) -> Self {
        RenderEntity(entity)
    }
}
impl ContainsEntity for RenderEntity {
    fn entity(&self) -> Entity {
        self.id()
    }
}
unsafe impl EntityEquivalent for RenderEntity {}


/// Component added on the Render world entities that are synced from the Main World in order to keep track of the corresponding main world entity.
/// It points to the corresponding entity in the main world.
#[derive(Component, Deref, Copy, Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Reflect)]
#[reflect(Component, Clone)]
pub struct MainEntity(pub(crate) Entity);
impl MainEntity {
    #[inline]
    pub fn id(&self) -> Entity {
        self.0
    }
}
impl From<Entity> for MainEntity {
    fn from(entity: Entity) -> Self {
        MainEntity(entity)
    }
}
impl ContainsEntity for MainEntity {
    fn entity(&self) -> Entity {
        self.id()
    }
}
unsafe impl EntityEquivalent for MainEntity {}



/// This module exists to keep the complex unsafe code out of the main module.
///
/// The implementations for both [`MainEntity`] and [`RenderEntity`] should stay in sync,
/// and are based off of the `&T` implementation in `bevy::ecs`.
mod render_entities_world_query_impls {
    use super::{MainEntity, RenderEntity};

    use bevy::ecs::{
        archetype::Archetype,
        change_detection::Tick,
        component::{ComponentId, Components},
        entity::Entity,
        query::{
            ArchetypeQueryData, FilteredAccess, QueryData, ReadOnlyQueryData,
            ReleaseStateQueryData, WorldQuery,
        },
        storage::{Table, TableRow},
        world::{World, unsafe_world_cell::UnsafeWorldCell},
    };

    // SAFETY: defers completely to `&RenderEntity` implementation,
    // and then only modifies the output safely.
    unsafe impl WorldQuery for RenderEntity {
        type Fetch<'w> = <&'static RenderEntity as WorldQuery>::Fetch<'w>;
        type State = <&'static RenderEntity as WorldQuery>::State;

        fn shrink_fetch<'wlong: 'wshort, 'wshort>(
            fetch: Self::Fetch<'wlong>,
        ) -> Self::Fetch<'wshort> {
            fetch
        }

        #[inline]
        unsafe fn init_fetch<'w>(
            world: UnsafeWorldCell<'w>,
            component_id: &ComponentId,
            last_run: Tick,
            this_run: Tick,
        ) -> Self::Fetch<'w> {
            // SAFETY: defers to the `&T` implementation, with T set to `RenderEntity`.
            unsafe {
                <&RenderEntity as WorldQuery>::init_fetch(world, component_id, last_run, this_run)
            }
        }

        const IS_DENSE: bool = <&'static RenderEntity as WorldQuery>::IS_DENSE;

        #[inline]
        unsafe fn set_archetype<'w>(
            fetch: &mut Self::Fetch<'w>,
            component_id: &ComponentId,
            archetype: &'w Archetype,
            table: &'w Table,
        ) {
            // SAFETY: defers to the `&T` implementation, with T set to `RenderEntity`.
            unsafe {
                <&RenderEntity as WorldQuery>::set_archetype(fetch, component_id, archetype, table);
            }
        }

        #[inline]
        unsafe fn set_table<'w>(
            fetch: &mut Self::Fetch<'w>,
            &component_id: &ComponentId,
            table: &'w Table,
        ) {
            // SAFETY: defers to the `&T` implementation, with T set to `RenderEntity`.
            unsafe { <&RenderEntity as WorldQuery>::set_table(fetch, &component_id, table) }
        }

        fn update_component_access(&component_id: &ComponentId, access: &mut FilteredAccess) {
            <&RenderEntity as WorldQuery>::update_component_access(&component_id, access);
        }

        fn init_state(world: &mut World) -> ComponentId {
            <&RenderEntity as WorldQuery>::init_state(world)
        }

        fn get_state(components: &Components) -> Option<Self::State> {
            <&RenderEntity as WorldQuery>::get_state(components)
        }

        fn matches_component_set(
            &state: &ComponentId,
            set_contains_id: &impl Fn(ComponentId) -> bool,
        ) -> bool {
            <&RenderEntity as WorldQuery>::matches_component_set(&state, set_contains_id)
        }
    }

    // SAFETY: Component access of Self::ReadOnly is a subset of Self.
    // Self::ReadOnly matches exactly the same archetypes/tables as Self.
    unsafe impl QueryData for RenderEntity {
        const IS_READ_ONLY: bool = true;
        const IS_ARCHETYPAL: bool = <&MainEntity as QueryData>::IS_ARCHETYPAL;
        type ReadOnly = RenderEntity;
        type Item<'w, 's> = Entity;

        fn shrink<'wlong: 'wshort, 'wshort, 's>(
            item: Self::Item<'wlong, 's>,
        ) -> Self::Item<'wshort, 's> {
            item
        }

        #[inline(always)]
        unsafe fn fetch<'w, 's>(
            state: &'s Self::State,
            fetch: &mut Self::Fetch<'w>,
            entity: Entity,
            table_row: TableRow,
        ) -> Option<Self::Item<'w, 's>> {
            // SAFETY: defers to the `&T` implementation, with T set to `RenderEntity`.
            let component =
                unsafe { <&RenderEntity as QueryData>::fetch(state, fetch, entity, table_row) };
            component.map(RenderEntity::id)
        }

        fn iter_access(
            state: &Self::State,
        ) -> impl Iterator<Item = bevy::ecs::query::EcsAccessType<'_>> {
            <&RenderEntity as QueryData>::iter_access(state)
        }
    }

    // SAFETY: the underlying `Entity` is copied, and no mutable access is provided.
    unsafe impl ReadOnlyQueryData for RenderEntity {}

    impl ArchetypeQueryData for RenderEntity {}

    impl ReleaseStateQueryData for RenderEntity {
        fn release_state<'w>(item: Self::Item<'w, '_>) -> Self::Item<'w, 'static> {
            item
        }
    }

    // SAFETY: defers completely to `&RenderEntity` implementation,
    // and then only modifies the output safely.
    unsafe impl WorldQuery for MainEntity {
        type Fetch<'w> = <&'static MainEntity as WorldQuery>::Fetch<'w>;
        type State = <&'static MainEntity as WorldQuery>::State;

        fn shrink_fetch<'wlong: 'wshort, 'wshort>(
            fetch: Self::Fetch<'wlong>,
        ) -> Self::Fetch<'wshort> {
            fetch
        }

        #[inline]
        unsafe fn init_fetch<'w>(
            world: UnsafeWorldCell<'w>,
            component_id: &ComponentId,
            last_run: Tick,
            this_run: Tick,
        ) -> Self::Fetch<'w> {
            // SAFETY: defers to the `&T` implementation, with T set to `MainEntity`.
            unsafe {
                <&MainEntity as WorldQuery>::init_fetch(world, component_id, last_run, this_run)
            }
        }

        const IS_DENSE: bool = <&'static MainEntity as WorldQuery>::IS_DENSE;

        #[inline]
        unsafe fn set_archetype<'w, 's>(
            fetch: &mut Self::Fetch<'w>,
            component_id: &ComponentId,
            archetype: &'w Archetype,
            table: &'w Table,
        ) {
            // SAFETY: defers to the `&T` implementation, with T set to `MainEntity`.
            unsafe {
                <&MainEntity as WorldQuery>::set_archetype(fetch, component_id, archetype, table);
            }
        }

        #[inline]
        unsafe fn set_table<'w>(
            fetch: &mut Self::Fetch<'w>,
            &component_id: &ComponentId,
            table: &'w Table,
        ) {
            // SAFETY: defers to the `&T` implementation, with T set to `MainEntity`.
            unsafe { <&MainEntity as WorldQuery>::set_table(fetch, &component_id, table) }
        }

        fn update_component_access(&component_id: &ComponentId, access: &mut FilteredAccess) {
            <&MainEntity as WorldQuery>::update_component_access(&component_id, access);
        }

        fn init_state(world: &mut World) -> ComponentId {
            <&MainEntity as WorldQuery>::init_state(world)
        }

        fn get_state(components: &Components) -> Option<Self::State> {
            <&MainEntity as WorldQuery>::get_state(components)
        }

        fn matches_component_set(
            &state: &ComponentId,
            set_contains_id: &impl Fn(ComponentId) -> bool,
        ) -> bool {
            <&MainEntity as WorldQuery>::matches_component_set(&state, set_contains_id)
        }
    }

    // SAFETY: Component access of Self::ReadOnly is a subset of Self.
    // Self::ReadOnly matches exactly the same archetypes/tables as Self.
    unsafe impl QueryData for MainEntity {
        const IS_READ_ONLY: bool = true;
        const IS_ARCHETYPAL: bool = <&MainEntity as QueryData>::IS_ARCHETYPAL;
        type ReadOnly = MainEntity;
        type Item<'w, 's> = Entity;

        fn shrink<'wlong: 'wshort, 'wshort, 's>(
            item: Self::Item<'wlong, 's>,
        ) -> Self::Item<'wshort, 's> {
            item
        }

        #[inline(always)]
        unsafe fn fetch<'w, 's>(
            state: &'s Self::State,
            fetch: &mut Self::Fetch<'w>,
            entity: Entity,
            table_row: TableRow,
        ) -> Option<Self::Item<'w, 's>> {
            // SAFETY: defers to the `&T` implementation, with T set to `MainEntity`.
            let component =
                unsafe { <&MainEntity as QueryData>::fetch(state, fetch, entity, table_row) };
            component.map(MainEntity::id)
        }

        fn iter_access(
            state: &Self::State,
        ) -> impl Iterator<Item = bevy::ecs::query::EcsAccessType<'_>> {
            <&MainEntity as QueryData>::iter_access(state)
        }
    }

    // SAFETY: the underlying `Entity` is copied, and no mutable access is provided.
    unsafe impl ReadOnlyQueryData for MainEntity {}

    impl ArchetypeQueryData for MainEntity {}

    impl ReleaseStateQueryData for MainEntity {
        fn release_state<'w>(item: Self::Item<'w, '_>) -> Self::Item<'w, 'static> {
            item
        }
    }
}
