use bevy::prelude::*;
use uuid::Uuid;
use wde_pbr::prelude::*;
use wde_renderer::prelude::*;

use crate::selection_area::material::SelectionAreaMaterial;

#[derive(Clone, Copy)]
pub(crate) struct ExtractedSelectionAreaInstance {
    pub mesh: AssetId<Mesh>,
    pub transform_uuid: Uuid,
    pub material: AssetId<SelectionAreaMaterial>
}

#[derive(Resource, Default)]
pub(crate) struct ExtractedSelectionAreaInstances(pub Vec<ExtractedSelectionAreaInstance>);

pub(crate) fn extract_selection_area_instances(
    selection_area: ExtractWorld<
        Query<(
            &Mesh3d,
            &PbrSsboTransformUuid,
            &PbrMaterial3d<SelectionAreaMaterial>
        )>
    >,
    mut extracted: ResMut<ExtractedSelectionAreaInstances>
) {
    extracted.0.clear();
    extracted.0.extend(
        selection_area
            .iter()
            .map(
                |(mesh, transform_uuid, material)| ExtractedSelectionAreaInstance {
                    mesh: mesh.0.id(),
                    transform_uuid: transform_uuid.0,
                    material: material.0.id()
                }
            )
    );
}
