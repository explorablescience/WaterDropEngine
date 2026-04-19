use bevy::prelude::*;
use uuid::Uuid;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

use crate::material::OutlineMaterial;

#[derive(Clone, Copy)]
pub(crate) struct ExtractedOutlineInstance {
    pub mesh: AssetId<Mesh>,
    pub transform_uuid: Uuid,
    pub material: AssetId<OutlineMaterial>
}

#[derive(Resource, Default)]
pub(crate) struct ExtractedOutlineInstances(pub Vec<ExtractedOutlineInstance>);

pub(crate) fn extract_outline_instances(
    selected: ExtractWorld<
        Query<(
            &Mesh3d,
            &PbrSsboTransformUuid,
            &PbrMaterial3d<OutlineMaterial>
        )>
    >,
    mut extracted: ResMut<ExtractedOutlineInstances>
) {
    extracted.0.clear();
    extracted
        .0
        .extend(selected.iter().map(
            |(mesh, transform_uuid, material)| ExtractedOutlineInstance {
                mesh: mesh.0.id(),
                transform_uuid: transform_uuid.0,
                material: material.0.id()
            }
        ));
}
