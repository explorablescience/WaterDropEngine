use wde_renderer::prelude::*;
use bevy::prelude::*;


/// The maximum number of entities in the ssbo.
pub(crate) const MAX_ENTITY_COUNT: usize = 100_000;

pub(crate) struct PbrSsboPlugin;
impl Plugin for PbrSsboPlugin {
    fn build(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Render, PbrSsbo::build_bind_group.in_set(RenderSet::BindGroups));
    }

    fn finish(&self, app: &mut App) {
        // Create the ssbo buffers
        let buffer: Handle<Buffer> = app.world_mut().add_asset(Buffer {
            label: "pbr-ssbo-cpu".to_string(),
            size: std::mem::size_of::<TransformUniform>() * MAX_ENTITY_COUNT,
            usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
            content: None,
        });
        let buffer_gpu: Handle<Buffer> = app.world_mut().add_asset(Buffer {
            label: "pbr-ssbo-gpu".to_string(),
            size: std::mem::size_of::<TransformUniform>() * MAX_ENTITY_COUNT,
            usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
            content: None,
        });

        // Create the instance to transform buffer
        let instance_to_transform_buffer: Handle<Buffer> = app.world_mut().add_asset(Buffer {
            label: "pbr-instance-to-transform-buffer-cpu".to_string(),
            size: std::mem::size_of::<u32>() * MAX_ENTITY_COUNT,
            usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
            content: None,
        });
        let instance_to_transform_buffer_gpu: Handle<Buffer> = app.world_mut().add_asset(Buffer {
            label: "pbr-instance-to-transform-buffer-gpu".to_string(),
            size: std::mem::size_of::<u32>() * MAX_ENTITY_COUNT,
            usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
            content: None,
        });

        app.get_sub_app_mut(RenderApp).unwrap()
            .world_mut().insert_resource(PbrSsbo {
                buffer_staging: buffer,
                buffer_gpu,
                instance_to_transform_buffer,
                instance_to_transform_buffer_gpu,
                bind_group_layout: None,
                bind_group: None
            });
    }
}


/// Store the SSBO of all model transforms
#[derive(Resource)]
pub struct PbrSsbo {
    /// The ssbo staging buffer used to update the gpu buffer
    pub buffer_staging: Handle<Buffer>,
    /// The ssbo gpu buffer where each transform is stored
    pub buffer_gpu: Handle<Buffer>,
    
    /// The instance-to-transform buffer
    pub instance_to_transform_buffer: Handle<Buffer>,
    /// The instance-to-transform gpu buffer
    pub instance_to_transform_buffer_gpu: Handle<Buffer>,

    // The bind group layout and bind group for the ssbo
    pub bind_group_layout: Option<BindGroupLayout>,
    pub bind_group: Option<BindGroup>
}
impl PbrSsbo {
    pub fn build_bind_group(buffers: Res<RenderAssets<GpuBuffer>>, mut ssbo: ResMut<PbrSsbo>, render_instance: Res<RenderInstance>) {
        // Check if the ssbo bind group is already created
        if ssbo.bind_group.is_some() {
            return;
        }

        // Get the ssbo buffer
        let buffer = match buffers.get(&ssbo.buffer_gpu) {
            Some(buffer) => buffer,
            None => return
        };

        // Get the instance-to-transform buffer
        let instance_to_transform_buffer = match buffers.get(&ssbo.instance_to_transform_buffer_gpu) {
            Some(buffer) => buffer,
            None => return
        };

        // Create the ssbo layout
        let ssbo_layout = BindGroupLayout::new("pbr-ssbo", |builder| {
            builder.add_buffer(0,
                ShaderStages::VERTEX,
                BufferBindingType::Storage { read_only: true });
            builder.add_buffer(1,
                ShaderStages::VERTEX,
                BufferBindingType::Storage { read_only: true });
        });
        let ssbo_layout_built = ssbo_layout.build(&render_instance.0.read().unwrap());

        // Create the bind group
        let render_instance = render_instance.0.read().unwrap();
        let bind_group = BindGroupBuilder::build("pbr-ssbo", &render_instance, &ssbo_layout_built, &vec![
            BindGroupBuilder::buffer(0, &buffer.buffer),
            BindGroupBuilder::buffer(1, &instance_to_transform_buffer.buffer)
        ]);
        ssbo.bind_group_layout = Some(ssbo_layout);
        ssbo.bind_group = Some(bind_group);
    }
}

