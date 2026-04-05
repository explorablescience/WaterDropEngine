use std::marker::PhantomData;

use bevy::prelude::*;

use crate::core::{Extract, ExtractWorld, RenderApp};

/// Plugin for extracting a resource from the main world to be used in the render world.
/// This plugin will automatically add a system to extract the resource of the corresponding type from the main world and insert it into the render world.
///
/// Note:
/// - The resource to extract must implement the [`ExtractResource`] trait.
/// - If the resource already exists in the render world, it will be updated with the new value from the main world. Otherwise, it will be inserted as a new resource in the render world.
/// - The update will only happen if the resource in the main world has changed, to avoid unnecessary updates and potential performance issues.
/// - The marker type `F` is only used as a way to bypass the orphan rules.
pub struct ExtractResourcePlugin<R: ExtractResource<F>, F = ()>(PhantomData<(R, F)>);
impl<R: ExtractResource<F>, F> Default for ExtractResourcePlugin<R, F> {
    fn default() -> Self {
        Self(PhantomData)
    }
}
impl<R: ExtractResource<F>, F: 'static + Send + Sync> Plugin for ExtractResourcePlugin<R, F> {
    fn build(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .add_systems(Extract, extract::<R, F>);
    }
}

/// Trait for extracting a resource from the main world to be used in the render world. See [`ExtractResourcePlugin`].
pub trait ExtractResource<F = ()>: Resource {
    type Source: Resource;

    /// Extracts the resource from the main world to be used in the render world.
    fn extract(source: &Self::Source) -> Self;
}

/// Extracts the resource of the corresponding [`Resource`] type
fn extract<R: ExtractResource<F>, F>(
    mut commands: Commands,
    main_resource: ExtractWorld<Option<Res<R::Source>>>,
    render_resource: Option<ResMut<R>>
) {
    if let Some(main_resource) = main_resource.as_ref() {
        if let Some(mut render_resource) = render_resource {
            if main_resource.is_changed() {
                *render_resource = R::extract(main_resource);
            }
        } else {
            commands.insert_resource(R::extract(main_resource));
        }
    }
}
