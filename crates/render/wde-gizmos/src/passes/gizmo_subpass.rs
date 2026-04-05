use bevy::{ecs::system::{SystemParamItem, lifetimeless::SRes}, platform::collections::HashMap, prelude::*};
use wde_camera::prelude::*;
use wde_renderer::prelude::*;

use crate::assets::gizmo_material::{GizmoMaterial, GizmoMaterialAsset};

use super::{GizmoSsbo, GpuGizmoRenderPipeline};

pub struct GizmoRenderBatch {
    #[allow(dead_code)]
    mesh: Handle<Mesh>,
    material: Handle<GizmoMaterialAsset>,
    first: usize,
    count: usize,
    index_count: usize,
}
#[derive(Resource, Default)]
pub struct GizmoRenderSubPass {
    /// The order of the batches: (mesh, material) -> [batch index].
    pub batches_order: HashMap<(AssetId<Mesh>, AssetId<GizmoMaterialAsset>), Vec<usize>>,
    /// The render batches.
    pub batches: Vec<GizmoRenderBatch>,
}
impl GizmoRenderSubPass {
    pub fn extract(
        main_entities: ExtractWorld<Query<(&Transform, &Mesh3d, &GizmoMaterial)>>,
        gizmo_ssbo: Res<GizmoSsbo>,
        mut gizmo_render_pass: ResMut<GizmoRenderSubPass>,
        meshes: Res<RenderAssets<GpuMesh>>,
        materials: Res<RenderAssets<GpuMaterial<GizmoMaterialAsset>>>,
        buffers: Res<RenderAssets<GpuBuffer>>,
        render_instance: Res<RenderInstance>,
    ) {
        // Get the ssbo
        let ssbo_bf = match buffers.get(&gizmo_ssbo.buffer) {
            Some(ssbo) => ssbo,
            None => return
        };
        
        // If no entities, clear the batches and return early
        if main_entities.iter().count() == 0 {
            gizmo_render_pass.batches_order.clear();
            gizmo_render_pass.batches.clear();
            return
        }

        // Create the batches
        let mut passes = GizmoRenderSubPass {
            batches_order: Default::default(),
            batches: Default::default()
        };
        {
            let render_instance = render_instance.0.read().unwrap();
            ssbo_bf.buffer.map_write(&render_instance, |mut view| {
                let mut first = 0;
                let mut count = 1;
                let mut last_mesh: Option<Handle<Mesh>> = None;
                let mut last_material: Option<Handle<GizmoMaterialAsset>> = None;
                let data = view.as_mut_ptr() as *mut TransformUniform;

                for (transform, mesh, material) in main_entities.iter() {
                    // Check if new element in same batch
                    let last_mesh_ref = last_mesh.as_ref();
                    let last_material_ref = last_material.as_ref();
                    if let (Some(last_mesh_ref), Some(last_material_ref)) = (last_mesh_ref, last_material_ref) {
                        if mesh.0.id() == last_mesh_ref.id() && material.0.id() == last_material_ref.id() {
                            // Update the ssbo
                            let transform = TransformUniform::new(transform);
                            unsafe {
                                *data.add(first + count) = transform;
                            }

                            // Increment the count
                            count += 1;

                            continue;
                        } else {
                            // Push the batch
                            passes.batches.push(GizmoRenderBatch {
                                mesh: last_mesh_ref.clone(),
                                material: last_material_ref.clone(),
                                first,
                                count,
                                index_count: match meshes.get(last_mesh_ref) {
                                    Some(mesh) => mesh.index_count as usize,
                                    None => 0
                                }
                            });
                            let batch_index = passes.batches.len() - 1;
                            passes.batches_order
                                .entry((last_mesh_ref.id(), last_material_ref.id()))
                                .or_default()
                                .push(batch_index);

                            // Reset the batch
                            first += count;
                            count = 1;
                            last_mesh = None;
                            last_material = None;
                        }
                    }

                    // Update the last mesh and ssbo if loaded
                    let mut updated_mesh = false;
                    let mut updated_material = false;
                    if meshes.get(&mesh.0).is_some() {
                        last_mesh = Some(mesh.0.clone());
                        updated_mesh = true;
                    }
                    if materials.get(&material.0).is_some() {
                        last_material = Some(material.0.clone());
                        updated_material = true;
                    }
                    if updated_mesh && updated_material {
                        // Update the ssbo
                        let transform = TransformUniform::new(transform);
                        unsafe {
                            *data.add(first) = transform;
                        }
                    }
                }

                // Push the last batch
                if let (Some(last_mesh), Some(last_material)) = (last_mesh, last_material) {
                    passes.batches.push(GizmoRenderBatch {
                        mesh: last_mesh.clone(),
                        material: last_material.clone(),
                        first,
                        count,
                        index_count: match meshes.get(&last_mesh) {
                            Some(mesh) => mesh.index_count as usize,
                            None => 0
                        }
                    });

                    let batch_index = passes.batches.len() - 1;
                    passes.batches_order.entry(
                        (last_mesh.id(), last_material.id())
                    ).or_default().push(batch_index);
                }
            });

            // Update the ssbo
            let ssbo_gpu = match buffers.get(&gizmo_ssbo.buffer_gpu) {
                Some(ssbo) => ssbo,
                None => return
            };
            ssbo_gpu.buffer.copy_from_buffer(&render_instance, &ssbo_bf.buffer);
        }

        // Update the passes
        *gizmo_render_pass = passes;
    }
}
impl RenderSubPass for GizmoRenderSubPass {
    type Params = (
        SRes<RenderAssets<GpuGizmoRenderPipeline>>, SRes<CameraFeatureRender>, SRes<GizmoSsbo>,
        SMaterial<GizmoMaterialAsset>, SRes<GizmoRenderSubPass>
    );

    fn describe(
        (pipeline, camera, ssbo, materials, render_pass): &SystemParamItem<Self::Params>
    ) -> RenderSubPassDesc {
        // Create the batches commands
        let mut batches = vec![];
        for (_, batch_index) in render_pass.batches_order.iter() {
            for &batch_index in batch_index.iter() {
                let batch = render_pass.batches.get(batch_index).unwrap();
            
                // TODO: Set meshes
                // // Set the mesh
                // if old_mesh_id != Some(batch.mesh.id()) {
                //     let mesh = match meshes.get(&batch.mesh) {
                //         Some(mesh) => mesh,
                //         None => continue // Should not happen
                //     };

                //     // Set the mesh buffers
                //     render_pass.set_vertex_buffer(0, mesh.vertex_buffer.as_ref().unwrap());
                //     render_pass.set_index_buffer(mesh.index_buffer.as_ref().unwrap());
                //     old_mesh_id = Some(batch.mesh.id());
                // }

                let material = match materials.get(&batch.material) {
                    Some(material) => material,
                    None => continue // Should not happen
                };
                batches.push(DrawCommandsBatch {
                    bind_group: Some((2, material.bind_group.clone())),
                    index_range: 0..batch.index_count as u32,
                    instance_range: batch.first as u32..((batch.first + batch.count) as u32)
                });
            }
        }

        // Describe the subpass
        RenderSubPassDesc(vec![
            SubPassCommand::Pipeline(Some(pipeline.iter().next().map(|(_, p)| p.0)).flatten()),
            SubPassCommand::BindGroup(0, camera.bind_group.clone()),
            SubPassCommand::BindGroup(1, ssbo.bind_group.clone()),
            SubPassCommand::DrawBatches(batches)
        ])
    }

    fn label() -> &'static str { "gizmo" }
}
