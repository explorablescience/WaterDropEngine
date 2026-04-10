use wde_logger::prelude::*;

use bevy::prelude::*;
use wde_renderer::prelude::*;

use crate::deferred::lights::*;

/// Maximum number of lights in the scene.
pub const MAX_LIGHTS: usize = 64;

pub(crate) struct LightsBindingPlugin;
impl Plugin for LightsBindingPlugin {
    fn build(&self, app: &mut App) {
        // Add the render bindings
        app.add_plugins((
            RenderBindingPluginRegisterOld::<LightsBinding>::default(),
            RenderBindingPluginRegisterOld::<LightsBindingStaging>::default()
        ));

        // Add the systems to extract the lights from the world, and to update the lights buffer
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<ExtractedLights>()
            .add_systems(Extract, extract)
            .add_systems(Render, update_lights_buffer.in_set(RenderSet::Prepare));
    }
}

#[derive(Asset, Clone, TypePath, Default)]
pub struct LightsBinding;
impl RenderBindingOld for LightsBinding {
    fn describe(&self, builder: &mut RenderBindingBuilderOld) {
        builder.add_buffer(
            0,
            Buffer {
                label: "lights-buffer-gpu".to_string(),
                size: std::mem::size_of::<LightsStorageElement>() * MAX_LIGHTS,
                usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                content: None
            }
        );
    }

    fn label(&self) -> &'static str {
        "lights-binding"
    }
}

#[derive(Asset, Clone, TypePath, Default)]
struct LightsBindingStaging;
impl RenderBindingOld for LightsBindingStaging {
    fn describe(&self, builder: &mut RenderBindingBuilderOld) {
        builder.add_buffer(
            0,
            Buffer {
                label: "lights-buffer-staging".to_string(),
                size: std::mem::size_of::<LightsStorageElement>() * MAX_LIGHTS,
                usage: BufferUsage::COPY_SRC | BufferUsage::COPY_DST,
                content: None
            }
        );

        // This binding is only used for staging the buffer, so it doesn't need a bind group.
        builder.no_bind_group();
    }

    fn label(&self) -> &'static str {
        "lights-binding-staging"
    }
}

#[derive(Resource, Default)]
struct ExtractedLights {
    pub directional_lights: Vec<DirectionalLight>,
    pub point_lights: Vec<PointLight>,
    pub spot_lights: Vec<SpotLight>
}

fn extract(
    lights_directional: ExtractWorld<Query<&DirectionalLight>>,
    lights_point: ExtractWorld<Query<&PointLight>>,
    lights_spot: ExtractWorld<Query<&SpotLight>>,
    mut extracted_lights: ResMut<ExtractedLights>
) {
    // Extract lights each frame. This is necessary to keep the lights buffer up to date, and to handle dynamic lights.
    extracted_lights.directional_lights = lights_directional.iter().copied().collect();
    extracted_lights.point_lights = lights_point.iter().copied().collect();
    extracted_lights.spot_lights = lights_spot.iter().copied().collect();
}

fn update_lights_buffer(
    lights_buffer: BindingOld<LightsBinding>,
    lights_buffer_staging: BindingOld<LightsBindingStaging>,
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
    let (lights_buffer, lights_buffer_staging) = match (
        lights_buffer.iter().next(),
        lights_buffer_staging.iter().next()
    ) {
        (Some((_, buffer)), Some((_, staging_buffer))) => (buffer, staging_buffer),
        _ => return
    };
    let lights_buffer_cpu = match buffers.get(lights_buffer_staging.get_buffer(0).unwrap()) {
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
    let lights_buffer_gpu = match buffers.get(lights_buffer.get_buffer(0).unwrap()) {
        Some(buffer) => buffer,
        None => return
    };
    lights_buffer_gpu
        .buffer
        .copy_from_buffer(&render_instance, &lights_buffer_cpu.buffer);
}
