use wde_logger::prelude::*;

use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::components::lights::*;

/// Maximum number of lights.
pub const MAX_LIGHTS: usize = 64;

/// Struct to hold the light uniform layout description.
#[derive(Resource)]
pub struct LightsFeatureBuffer {
    pub buffer_cpu: Handle<Buffer>,
    pub buffer_gpu: Handle<Buffer>,
    pub bind_group: Option<BindGroup>
}
impl LightsFeatureBuffer {
    pub fn build_bind_group(
        buffers: Res<RenderAssets<GpuBuffer>>,
        mut lights_buffer: ResMut<LightsFeatureBuffer>,
        render_instance: Res<RenderInstance>
    ) {
        // Check if the bind group is already created
        if lights_buffer.bind_group.is_some() {
            return;
        }

        // Get the lights buffer
        let buffer = match buffers.get(&lights_buffer.buffer_gpu) {
            Some(buffer) => buffer,
            None => return
        };

        // Create the bind group layout
        let layout_built = Self::layout()
            .build(&render_instance.0.read().unwrap())
            .unwrap();

        // Create the bind group
        let render_instance = render_instance.0.read().unwrap();
        let bind_group = BindGroupBuilder::build(
            "lights",
            &render_instance,
            &layout_built,
            &vec![BindGroupBuilder::buffer(0, &buffer.buffer)]
        )
        .unwrap();
        lights_buffer.bind_group = Some(bind_group);
    }

    pub fn layout() -> BindGroupLayout {
        BindGroupLayout::new("lights", |builder| {
            builder.add_buffer(
                0,
                ShaderStages::FRAGMENT,
                BufferBindingType::Storage { read_only: true }
            );
        })
    }
}

pub(crate) struct LightsFeature;
impl Plugin for LightsFeature {
    fn build(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .add_systems(Extract, extract)
            .add_systems(
                Render,
                LightsFeatureBuffer::build_bind_group.in_set(RenderSet::BindGroups)
            )
            .init_resource::<ExtractedLights>()
            .add_systems(Render, update_lights_buffer.in_set(RenderSet::Prepare));
    }

    fn finish(&self, app: &mut App) {
        let buffer_cpu: Handle<Buffer> = app.world_mut().add_asset(Buffer {
            label: "lights".to_string(),
            size: std::mem::size_of::<LightsStorageElement>() * MAX_LIGHTS,
            usage: BufferUsage::COPY_SRC | BufferUsage::COPY_DST,
            content: None
        });
        let buffer_gpu: Handle<Buffer> = app.world_mut().add_asset(Buffer {
            label: "lights".to_string(),
            size: std::mem::size_of::<LightsStorageElement>() * MAX_LIGHTS,
            usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
            content: None
        });

        // Add resources
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .insert_resource(LightsFeatureBuffer {
                buffer_cpu,
                buffer_gpu,
                bind_group: None
            });
    }
}

#[derive(Resource, Default)]
struct ExtractedLights {
    pub directional_lights: Vec<DirectionalLight>,
    pub point_lights: Vec<PointLight>,
    pub spot_lights: Vec<SpotLight>
}

fn extract(
    (lights_directional, lights_point, lights_spot): (
        ExtractWorld<Query<&DirectionalLight>>,
        ExtractWorld<Query<&PointLight>>,
        ExtractWorld<Query<&SpotLight>>
    ),
    mut extracted_lights: ResMut<ExtractedLights>
) {
    // Extract directional lights
    extracted_lights.directional_lights = lights_directional.iter().copied().collect();

    // Extract point lights
    extracted_lights.point_lights = lights_point.iter().copied().collect();

    // Extract spot lights
    extracted_lights.spot_lights = lights_spot.iter().copied().collect();
}

fn update_lights_buffer(
    lights_buffer: Res<LightsFeatureBuffer>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    render_instance: Res<RenderInstance>,
    extracted_lights: Res<ExtractedLights>,
    mut local_frame_counter: Local<bool>
) {
    // If even frame, skip updating to reduce overhead
    if *local_frame_counter {
        *local_frame_counter = false;
        return;
    } else {
        *local_frame_counter = true;
    }

    // Get the lights buffer
    let lights_buffer_cpu = match buffers.get(&lights_buffer.buffer_cpu) {
        Some(lights_buffer) => lights_buffer,
        None => return
    };

    let render_instance = render_instance.0.read().unwrap();
    let mut offset = 0;
    for light in extracted_lights.directional_lights.iter() {
        let data = LightsStorageElement::from_directional(light);
        lights_buffer_cpu.buffer.write(
            &render_instance,
            bytemuck::bytes_of(&data),
            offset * std::mem::size_of::<LightsStorageElement>()
        );
        offset += 1;
    }
    for light in extracted_lights.point_lights.iter() {
        let data = LightsStorageElement::from_point(light);
        lights_buffer_cpu.buffer.write(
            &render_instance,
            bytemuck::bytes_of(&data),
            offset * std::mem::size_of::<LightsStorageElement>()
        );
        offset += 1;
    }
    for light in extracted_lights.spot_lights.iter() {
        let data = LightsStorageElement::from_spot(light);
        lights_buffer_cpu.buffer.write(
            &render_instance,
            bytemuck::bytes_of(&data),
            offset * std::mem::size_of::<LightsStorageElement>()
        );
        offset += 1;
    }
    if offset > MAX_LIGHTS {
        warn!(
            "Number of lights exceeded the maximum of {}. Some lights will be ignored in rendering.",
            MAX_LIGHTS
        );
    }

    // Update the buffer
    let lights_buffer_gpu = match buffers.get(&lights_buffer.buffer_gpu) {
        Some(buffer) => buffer,
        None => return
    };
    lights_buffer_gpu
        .buffer
        .copy_from_buffer(&render_instance, &lights_buffer_cpu.buffer);
}
