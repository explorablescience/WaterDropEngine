use wde_logger::prelude::*;

use bevy::{ecs::system::SystemParamItem, prelude::*};
use wde_renderer::prelude::*;

use crate::deferred::lights::*;

/// Maximum number of lights in the scene.
pub const MAX_LIGHTS: usize = 64;

pub(crate) struct LightsBindingPlugin;
impl Plugin for LightsBindingPlugin {
    fn build(&self, app: &mut App) {
        // Add the render bindings
        app.add_plugins(RenderDataRegisterPlugin::<LightsData>::default());

        // Add the systems to extract the lights from the world, and to update the lights buffer
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<ExtractedLights>()
            .add_systems(Extract, extract)
            .add_systems(Render, update_lights_buffer.in_set(RenderSet::Prepare));
    }
}

#[derive(Asset, Clone, TypePath, Default)]
pub struct LightsData;
impl LightsData {
    pub const LIGHTS_BUFFER_IDX: u32 = 0;
    pub const LIGHTS_BUFFER_STAGING_IDX: u32 = 1;
}
impl RenderData for LightsData {
    type Params = ();

    fn describe(_params: &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        builder
            .add_buffer(
                Self::LIGHTS_BUFFER_IDX,
                Buffer {
                    label: "lights-buffer-gpu".to_string(),
                    size: std::mem::size_of::<LightsStorageElement>() * MAX_LIGHTS,
                    usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                    content: None
                }
            )
            .add_buffer(
                Self::LIGHTS_BUFFER_STAGING_IDX,
                Buffer {
                    label: "lights-buffer-staging".to_string(),
                    size: std::mem::size_of::<LightsStorageElement>() * MAX_LIGHTS,
                    usage: BufferUsage::COPY_SRC | BufferUsage::COPY_DST,
                    content: None
                }
            );
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
    lights_buffer: ResRenderData<LightsData>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    render_instance: Res<RenderInstance>,
    extracted_lights: Res<ExtractedLights>
) {
    // Get the lights buffer
    let lights_buffer_cpu = match lights_buffer.iter().next() {
        Some((_, buffer)) => match buffers.get(
            &buffer
                .get_buffer(LightsData::LIGHTS_BUFFER_STAGING_IDX)
                .unwrap()
        ) {
            Some(lights_buffer) => lights_buffer,
            None => return
        },
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
    let lights_buffer_gpu = match buffers.get(
        &lights_buffer
            .iter()
            .next()
            .unwrap()
            .1
            .get_buffer(LightsData::LIGHTS_BUFFER_IDX)
            .unwrap()
    ) {
        Some(buffer) => buffer,
        None => return
    };
    lights_buffer_gpu
        .buffer
        .copy_from_buffer(&render_instance, &lights_buffer_cpu.buffer);
}
