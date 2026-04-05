//! Macros for extracting data from the main world to the render world.
//! Provides the [`ExtractWorld`] system parameter, which allows accessing data from the main world in the render world.

use bevy::{
    ecs::{
        change_detection::Tick,
        system::{ReadOnlySystemParam, SystemMeta, SystemParam, SystemParamItem, SystemState},
        world::unsafe_world_cell::UnsafeWorldCell
    },
    prelude::*
};
use std::ops::{Deref, DerefMut};

use super::MainWorld;

/// A helper for accessing MainWorld content using a system parameter.
/// [`ExtractWorld`] is used to get data from the main world during [`crate::core::Extract`].
/// Note that the system parameter `P` must implement `ReadOnlySystemParam`, as the main world is only accessible for reading in this context.
///
/// ## Example
/// ```
/// fn extract(mut commands: Commands, clouds: ExtractWorld<Query<Entity, With<Cloud>>>) {
///     /* (...) */
/// }
/// ```
pub struct ExtractWorld<'w, 's, P>
where
    P: ReadOnlySystemParam + 'static
{
    item: SystemParamItem<'w, 's, P>
}
pub struct ExtractState<P: SystemParam + 'static> {
    state: SystemState<P>,
    main_world_state: <Res<'static, MainWorld> as SystemParam>::State
}
// SAFETY: The only `World` access (`Res<MainWorld>`) is read-only.
unsafe impl<P> ReadOnlySystemParam for ExtractWorld<'_, '_, P> where P: ReadOnlySystemParam {}
// SAFETY: The only `World` access is properly registered by `Res<MainWorld>::init_state`.
// This call will also ensure that there are no conflicts with prior params.
unsafe impl<P> SystemParam for ExtractWorld<'_, '_, P>
where
    P: ReadOnlySystemParam
{
    type State = ExtractState<P>;
    type Item<'w, 's> = ExtractWorld<'w, 's, P>;

    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut bevy::ecs::query::FilteredAccessSet,
        world: &mut World
    ) {
        // Register access for `Res<MainWorld>`.
        Res::<MainWorld>::init_access(
            &state.main_world_state,
            system_meta,
            component_access_set,
            world
        );
    }

    fn init_state(world: &mut World) -> Self::State {
        let mut main_world = world.resource_mut::<MainWorld>();
        ExtractState {
            state: SystemState::new(&mut main_world),
            main_world_state: Res::<MainWorld>::init_state(world)
        }
    }

    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick
    ) -> Self::Item<'w, 's> {
        // SAFETY:
        // - The caller ensures that `world` is the same one that `init_state` was called with.
        // - The caller ensures that no other `SystemParam`s will conflict with the accesses we have registered.
        let main_world = unsafe {
            Res::<MainWorld>::get_param(
                &mut state.main_world_state,
                system_meta,
                world,
                change_tick
            )
        };
        let item = state.state.get(main_world.into_inner());
        ExtractWorld { item }
    }
}
impl<'w, 's, P> Deref for ExtractWorld<'w, 's, P>
where
    P: ReadOnlySystemParam
{
    type Target = SystemParamItem<'w, 's, P>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.item
    }
}
impl<P> DerefMut for ExtractWorld<'_, '_, P>
where
    P: ReadOnlySystemParam
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.item
    }
}
impl<'a, 'w, 's, P> IntoIterator for &'a ExtractWorld<'w, 's, P>
where
    P: ReadOnlySystemParam,
    &'a SystemParamItem<'w, 's, P>: IntoIterator
{
    type Item = <&'a SystemParamItem<'w, 's, P> as IntoIterator>::Item;
    type IntoIter = <&'a SystemParamItem<'w, 's, P> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        (&self.item).into_iter()
    }
}
