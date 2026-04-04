use bevy::prelude::*;
use wde_wgpu::{buffer::{BufferBindingType, BufferUsage}, pipelines::{BindGroup, BindGroupBuilder, BindGroupLayout, ShaderStages}, utils::Vertex};

use crate::{assets::{Buffer, GpuBuffer, RenderAssets}, core::{Render, RenderApp, RenderInstance, RenderSet}};

const MAX_VERTICES: usize = 1_000_000;
const MAX_INDICES: usize = 3_000_000;

pub struct SsboMeshPlugin;
impl Plugin for SsboMeshPlugin {
    fn build(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp).unwrap()
            .init_resource::<SsboMesh>()
            .add_systems(Render, SsboMesh::build_bind_group.in_set(RenderSet::BindGroups));
    }

    fn finish(&self, app: &mut App) {
        // Create the vertex buffers
        let vertex_buffer: Handle<Buffer> = app.world_mut().add_asset(Buffer {
            label: "ssbo-mesh-vertex-buffer-gpu".to_string(),
            size: std::mem::size_of::<Vertex>() * MAX_VERTICES,
            usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
            content: None,
        });

        // Create the index buffers
        let index_buffer: Handle<Buffer> = app.world_mut().add_asset(Buffer {
            label: "ssbo-mesh-index-buffer-gpu".to_string(),
            size: std::mem::size_of::<u32>() * MAX_INDICES,
            usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
            content: None,
        });

        app.get_sub_app_mut(RenderApp).unwrap()
            .world_mut().insert_resource(SsboMesh {
                vertex_buffer,
                index_buffer,
                vertex_buffer_offset: 0,
                index_buffer_offset: 0,
                bind_group: None
            });
    }
}

/// Resource representing the SSBO mesh data, containing the vertex and index buffers, their offsets, and the bind group for rendering.
/// It is filled by every GpuMesh if `use_ssbo` is set to true.
/// The position of the vertex and index data in the buffers is then stored in the `GpuMesh` resource.
#[derive(Resource, Default)]
pub struct SsboMesh {
    // The ssbo buffers
    pub vertex_buffer: Handle<Buffer>,
    pub index_buffer: Handle<Buffer>,

    // Cursor to the current offset in the buffers (in elements, not bytes)
    pub vertex_buffer_offset: u32,
    pub index_buffer_offset: u32,

    // The bind group layout and bind group
    pub bind_group: Option<BindGroup>
}
impl SsboMesh {
    fn build_bind_group(buffers: Res<RenderAssets<GpuBuffer>>, mut ssbo: ResMut<SsboMesh>, render_instance: Res<RenderInstance>) {
        // Check if the ssbo bind group is already created
        if ssbo.bind_group.is_some() {
            return;
        }

        // Get the ssbo buffers
        let (vertex_buffer, index_buffer) = match (
            buffers.get(&ssbo.vertex_buffer),
            buffers.get(&ssbo.index_buffer)
        ) {
            (Some(vb), Some(ib)) => (vb, ib),
            _ => return
        };

        // Create the ssbo layout
        let ssbo_layout_built = SsboMesh::layout().build(&render_instance.0.read().unwrap());

        // Create the bind group
        let render_instance = render_instance.0.read().unwrap();
        let bind_group = BindGroupBuilder::build("ssbo-mesh", &render_instance, &ssbo_layout_built, &vec![
            BindGroupBuilder::buffer(0, &vertex_buffer.buffer),
            BindGroupBuilder::buffer(1, &index_buffer.buffer)
        ]);
        ssbo.bind_group = Some(bind_group);
    }

    pub fn layout() -> BindGroupLayout {
        BindGroupLayout::new("ssbo-mesh", |builder| {
            builder.add_buffer(0,
                ShaderStages::VERTEX,
                BufferBindingType::Storage { read_only: true });
            builder.add_buffer(1,
                ShaderStages::VERTEX,
                BufferBindingType::Storage { read_only: true });
        })
    }   
}
