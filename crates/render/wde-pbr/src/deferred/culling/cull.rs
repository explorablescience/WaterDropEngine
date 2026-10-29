use std::time::Instant;
use wde_logger::prelude::*;

use bevy::{ecs::entity::EntityHashMap, prelude::*};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use super::{
    CAMERA_VIEW, CULL_VIEW_COUNT, CullGroup, CullingSettings, CullingStats, CullingStatsShared,
    InCullGroup, shadow_view
};
use crate::prelude::{
    BatchList, CascadeEnabledFlags, CastShadow, ExtractedCascadedShadowParams,
    ExtractedPbrInstance, PbrMaterial, PbrSsboInstanceToTransform, SSBO_MAX_BATCHES,
    is_batch_blended
};

#[derive(Resource, Default)]
pub(crate) struct CameraFrustum(Option<Frustum>);

/// Non-empty bounds of the cull groups, by main world entity.
#[derive(Resource, Default)]
pub(crate) struct ExtractedCullGroups(EntityHashMap<Aabb>);

/// World-space bounds of a batched instance.
#[derive(Component)]
pub(crate) struct WorldBounds(Aabb);

pub(super) fn extract_camera_frustum(
    camera: ExtractWorld<Query<(&GlobalTransform, &CameraView), With<CullingCamera>>>,
    window: ExtractWorld<Query<&Window>>,
    mut frustum: ResMut<CameraFrustum>
) {
    frustum.0 = match (camera.single(), window.single()) {
        (Ok((transform, view)), Ok(window)) if window.height() > 0.0 => Some(
            Frustum::from_view_proj(view.world_to_ndc(transform, window.width() / window.height()))
        ),
        _ => None
    };
}

pub(super) fn extract_cull_groups(
    groups: ExtractWorld<Query<(Entity, &CullGroup)>>,
    mut extracted: ResMut<ExtractedCullGroups>
) {
    extracted.0.clear();
    extracted.0.extend(
        groups
            .iter()
            .filter(|(_, group)| !group.0.is_empty())
            .map(|(entity, group)| (entity, group.0))
    );
}

#[allow(clippy::type_complexity)]
pub(super) fn update_world_bounds(
    mut commands: Commands,
    meshes: Res<RenderAssets<GpuMesh>>,
    mut instances: Query<
        (
            Entity,
            &GlobalTransform,
            &ExtractedPbrInstance,
            Option<&mut WorldBounds>
        ),
        Or<(
            Changed<GlobalTransform>,
            Changed<ExtractedPbrInstance>,
            Without<WorldBounds>
        )>
    >
) {
    for (entity, transform, instance, bounds) in &mut instances {
        // Instances without bounds are retried each frame, and never culled meanwhile
        let Some(mesh) = meshes.get(instance.mesh_id) else {
            if bounds.is_some() {
                commands.entity(entity).remove::<WorldBounds>();
            }
            continue;
        };
        let aabb = Aabb::new(mesh.bounding_box.min, mesh.bounding_box.max).transformed(transform);
        match bounds {
            Some(mut bounds) => bounds.0 = aabb,
            None => {
                commands.entity(entity).insert(WorldBounds(aabb));
            }
        }
    }
}

/// Writes the visible instances of each batch in each view to the instance to transform buffer.
#[allow(clippy::too_many_arguments)]
pub(super) fn cull_instances(
    mut batches: ResMut<BatchList>,
    instance_to_transform: ResRenderData<PbrSsboInstanceToTransform>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    render_instance: Res<RenderInstance>,
    materials: Res<RenderAssets<GpuRenderBinding<PbrMaterial>>>,
    settings: Res<CullingSettings>,
    camera: Res<CameraFrustum>,
    cascades: Res<ExtractedCascadedShadowParams>,
    cascades_enabled: Res<CascadeEnabledFlags>,
    groups: Res<ExtractedCullGroups>,
    instances: Query<(Option<&WorldBounds>, Option<&InCullGroup>, Has<CastShadow>)>,
    shared_stats: Res<CullingStatsShared>,
    mut lists: Local<[Vec<u32>; CULL_VIEW_COUNT]>
) {
    let _span = debug_span!("pbr_culling").entered();
    let start = Instant::now();
    let Some(gpu_buffer) = instance_to_transform.iter().next().and_then(|(_, data)| {
        buffers.get(&data.get_buffer(PbrSsboInstanceToTransform::INSTANCE_TO_TRANSFORM_IDX)?)
    }) else {
        return;
    };

    // Views without a frustum draw everything
    let mut frustums: [Option<Frustum>; CULL_VIEW_COUNT] = [None; CULL_VIEW_COUNT];
    if settings.enabled {
        frustums[CAMERA_VIEW] = camera.0;
        if settings.shadows {
            for (cascade, params) in cascades.0.iter().enumerate() {
                frustums[shadow_view(cascade as u32)] = Some(Frustum::from_view_proj(
                    Mat4::from_cols_array_2d(&params.view_proj)
                ));
            }
        }
    }

    let mut stats = CullingStats {
        instances: batches.tracked_entities.len(),
        groups: groups.0.len(),
        ..default()
    };
    let group_tests: EntityHashMap<[FrustumTest; CULL_VIEW_COUNT]> = match settings.groups {
        true => groups
            .0
            .iter()
            .map(|(entity, aabb)| {
                let tests = frustums.map(|f| f.map_or(FrustumTest::Inside, |f| f.test(aabb)));
                for (view, test) in tests.iter().enumerate() {
                    stats.groups_culled[view] += (*test == FrustumTest::Outside) as usize;
                }
                (*entity, tests)
            })
            .collect(),
        false => default()
    };

    for list in lists.iter_mut() {
        list.clear();
    }
    for (key, batch) in batches.batches.iter_mut() {
        let shadowed = !is_batch_blended(&materials, key);
        let starts = lists.each_ref().map(|list| list.len() as u32);
        for &(entity, transform_id) in &batch.instances {
            let (bounds, group, cast_shadow) = instances.get(entity).unwrap_or((None, None, false));
            let group = group.and_then(|g| group_tests.get(&g.0));
            for (view, list) in lists.iter_mut().enumerate() {
                if view != CAMERA_VIEW
                    && !(shadowed && cast_shadow && cascades_enabled.0[view - shadow_view(0)])
                {
                    continue;
                }
                let visible = match (frustums[view], group.map(|tests| tests[view])) {
                    (None, _) | (_, Some(FrustumTest::Inside)) => true,
                    (_, Some(FrustumTest::Outside)) => false,
                    (Some(frustum), _) => match (settings.meshes, bounds) {
                        (true, Some(bounds)) => {
                            stats.mesh_tests += 1;
                            frustum.intersects(&bounds.0)
                        }
                        _ => true
                    }
                };
                if visible {
                    list.push(transform_id);
                }
            }
        }
        batch.views = std::array::from_fn(|view| starts[view]..lists[view].len() as u32);
    }

    // The views follow each other in the buffer
    let mut offset = 0;
    let offsets: [u32; CULL_VIEW_COUNT] = std::array::from_fn(|view| {
        let base = offset;
        offset += lists[view].len() as u32;
        base
    });
    if offset as usize > SSBO_MAX_BATCHES * CULL_VIEW_COUNT {
        error!(
            "Too many visible PBR instances ({}), the limit is {}.",
            offset,
            SSBO_MAX_BATCHES * CULL_VIEW_COUNT
        );
        for batch in batches.batches.values_mut() {
            batch.views = Default::default();
        }
        return;
    }
    for batch in batches.batches.values_mut() {
        for (range, base) in batch.views.iter_mut().zip(offsets) {
            *range = range.start + base..range.end + base;
        }
    }
    let render_instance = render_instance.0.read().unwrap();
    for (list, base) in lists.iter().zip(offsets) {
        if !list.is_empty() {
            gpu_buffer.buffer.write(
                &render_instance,
                bytemuck::cast_slice(list),
                base as usize * std::mem::size_of::<u32>()
            );
        }
    }

    stats.drawn = lists.each_ref().map(|list| list.len());
    let micros = start.elapsed().as_secs_f32() * 1e6;
    if let Ok(mut shared) = shared_stats.0.lock() {
        stats.micros = shared.micros * 0.95 + micros * 0.05;
        *shared = stats;
    }
}
