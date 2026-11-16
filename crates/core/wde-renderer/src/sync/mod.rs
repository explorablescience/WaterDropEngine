//! The sync module provides tools to synchronize entities, components and resources from the main world to the render world. This is useful for plugins that need to extract data from the main world and use it in the render world.
//!
//! It is built on [`bevy::extract`], specialized for the [`RenderApp`]: the traits take [`RenderApp`] as their label, and the
//! plugins and entity components are aliased (e.g. [`RenderEntity`] is `SubEntity<RenderApp>`).
//!
//!
//! # Extracting resources
//! ## Manual extraction
//! If you have a plugin that has resources that need to be extracted, you can implement the [`ExtractResource`] trait for those resources and add the corresponding [`ExtractResourcePlugin`] to your plugin. For example, if you have a plugin with a resource `MyResource`, you can do:
//! ```
//! app
//!     .insert_resource(MyResource { /* ... */ })
//!     .add_plugins(ExtractResourcePlugin::<MyResource>::default());
//! ```
//! This will automatically extract `MyResource` from the main world to the render world, and update it in the render world whenever it changes in the main world, without having to write the extraction system manually. The associated [`ExtractResource`] trait implementation will look like this:
//! ```
//! impl ExtractResource<RenderApp> for MyResource {
//!    type Source = Self;
//!
//!    fn extract_resource(source: &Self::Source) -> Self {
//!       source.clone()
//!    }
//! }
//! ```
//!
//! ## ExtractResource macro
//! If the extraction logic is simple, (i.e. when it only consists of cloning the resource), the trait implementation can be automatically derived using the [`ExtractResource`] derive macro. For example,
//! ```
//! #[derive(Resource, Clone, ExtractResource)]
//! #[extract_app(RenderApp)]
//! struct MyResource {
//!    value: i32,
//! }
//! ```
//!
//!
//! # Syncing entities and components
//! Entities from the main world can be synchronized to the render world by adding the [`SyncToRenderWorld`] component to them.
//!
//! ## Manual synchronization vs automatic synchronization
//! One way to do this is to add the component manually to the entities that need to be synchronized. For example:
//! ```
//! commands.spawn((MyComponent { /* ... */ }, SyncToRenderWorld::default()));
//! ```
//! Another way to synchronize entities is to use the [`SyncComponentPlugin`] for a component that is on those entities. For example, if you have a component `MyComponent` that is on the entities you want to synchronize, you can do:
//! ```
//! app.add_plugins(SyncComponentPlugin::<MyComponent>::default());
//! ```
//! This will automatically add the [`SyncToRenderWorld`] component to any entity that has `MyComponent`, which will trigger the synchronization of that entity to the render world.
//!
//! ## Synchronized entities
//! After an entity is synchronized, it will have the following components in order to maintain the link between the main world and the render world:
//! - In the main world, a [`RenderEntity`] component that stores the corresponding entity in the render world.
//! - In the render world, a [`MainEntity`] component that stores the corresponding entity in the main world.
//!
//! You can access these components just like the [`Entity`](bevy::prelude::Entity) component (without &).
//! For example, you can then create `extract` systems that query for every entity with `MyComponent` that changed in the main world (supposing you used [`SyncComponentPlugin`] for `MyComponent`), and then extract these changes to their corresponding render world entities:
//! ```rust
//! fn extract_my_component(
//!    query: Extract<Query<(RenderEntity, &MyComponent), Changed<MyComponent>>>,
//!    mut commands: Commands
//! ) {
//!    for (render_entity, my_component) in query.iter() {
//!       commands
//!         .entity(render_entity)
//!         .insert(MyComponent { /* ... */ });
//!    }
//! }
//! ```
//! Note that one doesn't need to create this entity in the render world manually, as it is automatically managed by the synchronization system.
//!
//! ## Extracting components
//! To simplify the process of extracting components, like the `extract_my_component` system in the example above, you can implement the [`ExtractComponent`] trait for a component and add the corresponding [`ExtractComponentPlugin`] to your plugin.
//!
//! For example, to implement the `extract_my_component` system above, first add the plugin:
//! ```
//! app.add_plugins(ExtractComponentPlugin::<MyComponent>::default());
//! ```
//! This will automatically add the [`SyncComponentPlugin`] for `MyComponent`.
//! Then, you can implement the [`ExtractComponent`] and [`SyncComponent`] traits for `MyComponent`:
//! ```
//! impl SyncComponent<RenderApp> for MyComponent {
//!     type Target = Self;
//! }
//! impl ExtractComponent<RenderApp> for MyComponent {
//!     type QueryData = &'static Self;
//!     type QueryFilter = Changed<Self>;
//!     type Out = Self;
//!
//!     fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
//!         Some(item.clone())
//!         // Note: If return None, remove the component from the entity in the render world
//!     }
//! }
//! ```
//! This will automatically extract `MyComponent` from the main world to the render world for every entity that has `MyComponent` and for which `MyComponent` changed in the main world, without having to write the `extract_my_component` system manually.
//!
//! ## ExtractComponent macro
//! When the extraction logic is simple, (i.e. when it only consists of cloning the component), the trait implementation of [`ExtractComponent`] and [`SyncComponent`] can be automatically derived using the [`ExtractComponent`] derive macro. For example,
//! ```
//! #[derive(Component, Clone, ExtractComponent)]
//! #[extract_app(RenderApp)]
//! #[extract_component_filter(Changed<Self>)]
//! struct MyComponent {
//!     value: i32,
//! }
//! ```

use bevy::extract;

use crate::core::RenderApp;

pub use bevy::ecs::query::QueryItem;
pub use extract::{
    Extract, ExtractSchedule, MainEntity, MainWorld, extract_component::ExtractComponent,
    extract_resource::ExtractResource, sync_component::SyncComponent
};

/// Main world component pointing to the synchronized entity of the render world.
pub type RenderEntity = extract::SubEntity<RenderApp>;
/// Marker component that synchronizes its entity to the render world.
pub type SyncToRenderWorld = extract::SyncToSubWorld<RenderApp>;
/// Adds the [`SyncToRenderWorld`] marker to the entities holding a [`SyncComponent`].
pub type SyncComponentPlugin<C, F = ()> =
    extract::sync_component::SyncComponentPlugin<C, RenderApp, F>;
/// Extracts an [`ExtractComponent`] to the render world entities.
pub type ExtractComponentPlugin<C, F = ()> =
    extract::extract_component::ExtractComponentPlugin<C, RenderApp, F>;
/// Extracts an [`ExtractResource`] to the render world.
pub type ExtractResourcePlugin<R, F = ()> =
    extract::extract_resource::ExtractResourcePlugin<R, RenderApp, F>;
