use bevy::prelude::*;
use wde_renderer::prelude::*;

/// The maximum number of entities in the ssbo.
pub(crate) const MAX_ENTITY_COUNT: usize = 100_000;

pub(crate) struct PbrSsboPlugin;
impl Plugin for PbrSsboPlugin {
    fn build(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp).unwrap().add_systems(
            Render,
            SsboTransformPbr::build_bind_group.in_set(RenderSet::BindGroups)
        );
    }

    fn finish(&self, app: &mut App) {
        // Create the ssbo buffers
        let buffer: Handle<Buffer> = app.world_mut().add_asset(Buffer {
            label: "pbr-ssbo-cpu".to_string(),
            size: std::mem::size_of::<TransformUniform>() * MAX_ENTITY_COUNT,
            usage: BufferUsage::COPY_SRC | BufferUsage::COPY_DST,
            content: None
        });
        let buffer_gpu: Handle<Buffer> = app.world_mut().add_asset(Buffer {
            label: "pbr-ssbo-gpu".to_string(),
            size: std::mem::size_of::<TransformUniform>() * MAX_ENTITY_COUNT,
            usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
            content: None
        });

        // Create the instance to transform buffer
        let instance_to_transform_buffer: Handle<Buffer> = app.world_mut().add_asset(Buffer {
            label: "pbr-instance-to-transform-buffer-cpu".to_string(),
            size: std::mem::size_of::<u32>() * MAX_ENTITY_COUNT,
            usage: BufferUsage::COPY_SRC | BufferUsage::COPY_DST,
            content: None
        });
        let instance_to_transform_buffer_gpu: Handle<Buffer> = app.world_mut().add_asset(Buffer {
            label: "pbr-instance-to-transform-buffer-gpu".to_string(),
            size: std::mem::size_of::<u32>() * MAX_ENTITY_COUNT,
            usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
            content: None
        });

        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .world_mut()
            .insert_resource(SsboTransformPbr {
                buffer_staging: buffer,
                buffer_gpu,
                instance_to_transform_buffer,
                instance_to_transform_buffer_gpu,
                bind_group: None
            });
    }
}

/// Store the SSBO of all model transforms of every entity with a PbrModel component.
/// The correspondance between the transform in the SSBO and the entity is done in the `PbrModelRegistry` resource.
/// The SSBO is updated every frame with the transforms of the entities that have a PbrModel component and that are marked as dirty in the `DirtyTransforms` resource (that is for each entity that has a modified Transform).
#[derive(Resource)]
pub struct SsboTransformPbr {
    /// The ssbo staging buffer used to update the gpu buffer
    pub buffer_staging: Handle<Buffer>,
    /// The ssbo gpu buffer where each transform is stored
    pub buffer_gpu: Handle<Buffer>,

    /// The instance-to-transform buffer
    pub instance_to_transform_buffer: Handle<Buffer>,
    /// The instance-to-transform gpu buffer
    pub instance_to_transform_buffer_gpu: Handle<Buffer>,

    // The bind group layout and bind group for the ssbo
    pub bind_group: Option<BindGroup>
}
impl SsboTransformPbr {
    pub fn build_bind_group(
        buffers: Res<RenderAssets<GpuBuffer>>,
        mut ssbo: ResMut<SsboTransformPbr>,
        render_instance: Res<RenderInstance>
    ) {
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
        let instance_to_transform_buffer = match buffers.get(&ssbo.instance_to_transform_buffer_gpu)
        {
            Some(buffer) => buffer,
            None => return
        };

        // Create the ssbo layout
        let ssbo_layout_built = SsboTransformPbr::get_layout()
            .build(&render_instance.0.read().unwrap())
            .unwrap();

        // Create the bind group
        let render_instance = render_instance.0.read().unwrap();
        let bind_group = BindGroupBuilder::build(
            "pbr-ssbo",
            &render_instance,
            &ssbo_layout_built,
            &vec![
                BindGroupBuilder::buffer(0, &buffer.buffer),
                BindGroupBuilder::buffer(1, &instance_to_transform_buffer.buffer),
            ]
        )
        .unwrap();
        ssbo.bind_group = Some(bind_group);
    }

    pub fn get_layout() -> BindGroupLayout {
        BindGroupLayout::new("pbr-ssbo", |builder| {
            builder.add_buffer(
                0,
                ShaderStages::VERTEX,
                BufferBindingType::Storage { read_only: true }
            );
            builder.add_buffer(
                1,
                ShaderStages::VERTEX,
                BufferBindingType::Storage { read_only: true }
            );
        })
    }
}
