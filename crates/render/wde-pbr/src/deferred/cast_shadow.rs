use bevy::prelude::*;
use wde_renderer::prelude::*;

/// Marker component that makes an entity cast shadows from the directional sun light.
/// Add this to any entity that also has [`PbrBatchesMarker`](crate::deferred::PbrBatchesMarker).
#[derive(Component, Default, Reflect, Clone)]
#[reflect(Component)]
pub struct CastShadow;
impl SyncComponent for CastShadow {
    type Target = Self;
}
impl ExtractComponent for CastShadow {
    type QueryData = ();
    type QueryFilter = With<CastShadow>;
    type Out = Self;

    fn extract_component(_item: bevy::ecs::query::QueryItem<'_, '_, ()>) -> Option<Self::Out> {
        Some(CastShadow)
    }
}

pub struct CastShadowPlugin;
impl Plugin for CastShadowPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CastShadow>()
            .add_plugins(ExtractComponentPlugin::<CastShadow>::default());
    }
}
