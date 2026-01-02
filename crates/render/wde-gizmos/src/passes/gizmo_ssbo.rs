use bevy::prelude::*;
use wde_renderer::prelude::*;

/// The maximum number of entities in the ssbo.
const MAX_ENTITY_COUNT: usize = 100_000;

#[derive(Resource)]
pub(crate) struct GizmoSsbo {
    pub buffer: Handle<Buffer>,
    pub buffer_gpu: Handle<Buffer>,
    pub bind_group_layout: Option<BindGroupLayout>,
    pub bind_group: Option<BindGroup>
}
impl GizmoSsbo {
    pub fn build_bind_group(buffers: Res<RenderAssets<GpuBuffer>>, mut ssbo: ResMut<GizmoSsbo>, render_instance: Res<RenderInstance<'static>>) {
        // Check if the ssbo bind group is already created
        if ssbo.bind_group.is_some() {
            return;
        }

        // Get the ssbo buffer
        let buffer = match buffers.get(&ssbo.buffer_gpu) {
            Some(buffer) => buffer,
            None => return
        };

        // Create the ssbo layout
        let ssbo_layout = BindGroupLayout::new("gizmo-ssbo", |builder| {
            builder.add_buffer(0,
                ShaderStages::VERTEX,
                BufferBindingType::Storage { read_only: true });
        });
        let ssbo_layout_built = ssbo_layout.build(&render_instance.0.read().unwrap());

        // Create the bind group
        let render_instance = render_instance.0.read().unwrap();
        let bind_group = BindGroupBuilder::build("gizmo-ssbo", &render_instance, &ssbo_layout_built, &vec![
            BindGroupBuilder::buffer(0, &buffer.buffer)
        ]);
        ssbo.bind_group_layout = Some(ssbo_layout);
        ssbo.bind_group = Some(bind_group);
    }
}

pub(crate) struct GizmoSsboPlugin;
impl Plugin for GizmoSsboPlugin {
    fn build(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Render, GizmoSsbo::build_bind_group.in_set(RenderSet::BindGroups));
    }

    fn finish(&self, app: &mut bevy::app::App) {
        let buffer: Handle<Buffer> = app.world_mut().add_asset(Buffer {
            label: "gizmo-ssbo-cpu".to_string(),
            size: std::mem::size_of::<TransformUniform>() * MAX_ENTITY_COUNT,
            usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
            content: None,
        });
        let buffer_gpu: Handle<Buffer> = app.world_mut().add_asset(Buffer {
            label: "gizmo-ssbo-gpu".to_string(),
            size: std::mem::size_of::<TransformUniform>() * MAX_ENTITY_COUNT,
            usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
            content: None,
        });

        app.get_sub_app_mut(RenderApp).unwrap()
            .world_mut().insert_resource(GizmoSsbo {
                buffer,
                buffer_gpu,
                bind_group_layout: None,
                bind_group: None
            });
    }
}

