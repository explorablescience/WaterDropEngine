use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::components::lights::*;

/// Maximum number of lights.
pub(crate) const MAX_LIGHTS: usize = 64;

/// Struct to hold the light uniform layout description.
#[derive(Resource)]
pub(crate) struct LightsFeatureBuffer {
    pub buffer_cpu: Handle<Buffer>,
    pub buffer_gpu: Handle<Buffer>,
    pub bind_group_layout: Option<BindGroupLayout>,
    pub bind_group: Option<BindGroup>
}
impl LightsFeatureBuffer {
    pub fn build_bind_group(
        buffers: Res<RenderAssets<GpuBuffer>>, mut lights_buffer: ResMut<LightsFeatureBuffer>,
        render_instance: Res<RenderInstance<'static>>
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
        let layout = BindGroupLayout::new("lights", |builder| {
            builder.add_buffer(0,
                ShaderStages::FRAGMENT,
                BufferBindingType::Storage { read_only: true });
        });
        let layout_built = layout.build(&render_instance.0.read().unwrap());

        // Create the bind group
        let render_instance = render_instance.0.read().unwrap();
        let bind_group = BindGroupBuilder::build("lights", &render_instance, &layout_built, &vec![
            BindGroupBuilder::buffer(0, &buffer.buffer)
        ]);
        lights_buffer.bind_group_layout = Some(layout);
        lights_buffer.bind_group = Some(bind_group);
    }
}

pub(crate) struct LightsFeature;
impl Plugin for LightsFeature {
    fn build(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Extract, extract)
            .add_systems(Render, LightsFeatureBuffer::build_bind_group.in_set(RenderSet::BindGroups));
    }

    fn finish(&self, app: &mut App) {
        let buffer_cpu: Handle<Buffer> = app.world_mut().add_asset(Buffer {
            label: "lights".to_string(),
            size:  std::mem::size_of::<LightsStorageElement>() * MAX_LIGHTS,
            usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
            content: None,
        });
        let buffer_gpu: Handle<Buffer> = app.world_mut().add_asset(Buffer {
            label: "lights".to_string(),
            size:  std::mem::size_of::<LightsStorageElement>() * MAX_LIGHTS,
            usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
            content: None,
        });
        
        // Add resources
        app.get_sub_app_mut(RenderApp).unwrap()
            .insert_resource(LightsFeatureBuffer {
                buffer_cpu,
                buffer_gpu,
                bind_group_layout: None,
                bind_group: None
            });
    }
}

fn extract(
    (lights_directional, lights_point, lights_spot): (
        ExtractWorld<Query<&DirectionalLight>>, ExtractWorld<Query<&PointLight>>, ExtractWorld<Query<&SpotLight>>
    ), 
    (lights_buffer, buffers): (
        Res<LightsFeatureBuffer>, Res<RenderAssets<GpuBuffer>>
    ),
    render_instance: Res<RenderInstance<'static>>
) {
    // Get the lights buffer
    let lights_buffer_cpu = match buffers.get(&lights_buffer.buffer_cpu) {
        Some(lights_buffer) => lights_buffer,
        None => return
    };
    
    let render_instance = render_instance.0.read().unwrap();
    lights_buffer_cpu.buffer.map_write(&render_instance, |mut view| {
        let data = view.as_mut_ptr() as *mut LightsStorageElement;
        let mut offset = 0;
        let mut first_element = None;

        // Extract directional lights
        for light in lights_directional.iter() {
            let element = LightsStorageElement::from_directional(light);
            if first_element.is_none() { first_element = Some(element); }
            unsafe { *data.add(offset) = element; }
            offset += 1;
        }

        // Extract point lights
        for light in lights_point.iter() {
            let element = LightsStorageElement::from_point(light);
            if first_element.is_none() { first_element = Some(element); }
            unsafe { *data.add(offset) = element; }
            offset += 1;
        }

        // Extract spot lights
        for light in lights_spot.iter() {
            let element = LightsStorageElement::from_spot(light);
            if first_element.is_none() { first_element = Some(element); }
            unsafe { *data.add(offset) = element; }
            offset += 1;
        }

        // Set the number of lights
        let lights_number = offset as f32;
        if let Some(mut first_element) = first_element {
            first_element.position_number = [
                first_element.position_number[0], first_element.position_number[1],
                first_element.position_number[2], lights_number
            ];
            unsafe { *data.add(0) = first_element; }
        }

        // Warn if the number of lights exceeds the maximum
        if lights_number > MAX_LIGHTS as f32 {
            warn!("The number of lights exceeds the maximum of {}.", MAX_LIGHTS);
        }
    });

    // Update the buffer
    let lights_buffer_gpu = match buffers.get(&lights_buffer.buffer_gpu) {
        Some(buffer) => buffer,
        None => return
    };
    lights_buffer_gpu.buffer.copy_from_buffer(&render_instance, &lights_buffer_cpu.buffer);
}

