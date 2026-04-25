use wde_logger::prelude::*;

use bevy::{ecs::system::SystemParamItem, prelude::*};
use wde_renderer::prelude::*;

use crate::deferred::lights::*;
use crate::prelude::{AtmosphereParamsUniform, AtmosphereSettings};

/// Maximum number of lights in the scene.
pub const MAX_LIGHTS: usize = 64;

pub(crate) struct LightsBindingPlugin;
impl Plugin for LightsBindingPlugin {
    fn build(&self, app: &mut App) {
        // Add the render bindings
        app.init_resource::<AtmosphereSettings>()
            .register_type::<AtmosphereSettings>()
            .add_plugins(RenderDataRegisterPlugin::<LightsData>::default());

        // Add the systems to extract the lights from the world, and to update the lights buffer
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<ExtractedLights>()
            .init_resource::<AtmosphereParamsUniform>()
            .add_systems(Extract, extract)
            .add_systems(Extract, extract_atmosphere_params)
            .add_systems(Render, update_lights_buffer.in_set(RenderSet::Prepare));
    }
}

#[derive(Asset, Clone, TypePath, Default)]
pub struct LightsData;
impl LightsData {
    pub const LIGHTS_BUFFER_IDX: u32 = 0;
    pub const LIGHTS_BUFFER_STAGING_IDX: u32 = 1;
    pub const ATMOSPHERE_PARAMS_BUFFER_IDX: u32 = 2;
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
            )
            .add_buffer(
                Self::ATMOSPHERE_PARAMS_BUFFER_IDX,
                Buffer {
                    label: "atmosphere-params".to_string(),
                    size: std::mem::size_of::<AtmosphereParamsUniform>(),
                    usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                    content: Some(bytemuck::cast_slice(&[AtmosphereParamsUniform::default()]).to_vec())
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

fn extract_atmosphere_params(
    settings: ExtractWorld<Res<AtmosphereSettings>>,
    time: ExtractWorld<Res<Time>>,
    mut uniform: ResMut<AtmosphereParamsUniform>
) {
    let day_length = settings.day_length_seconds.max(1.0);
    let time_of_day = if settings.animate_time {
        (time.elapsed_secs() / day_length).fract()
    } else {
        settings.time_of_day.fract()
    };
    *uniform = AtmosphereParamsUniform::from_settings(&settings, time_of_day);
}

fn update_lights_buffer(
    lights_buffer: ResRenderData<LightsData>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    render_instance: Res<RenderInstance>,
    extracted_lights: Res<ExtractedLights>,
    atmosphere_uniform: Res<AtmosphereParamsUniform>
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

    // Update atmosphere uniform buffer stored in the same render data as lights.
    let atmosphere_buffer = match buffers.get(
        &lights_buffer
            .iter()
            .next()
            .unwrap()
            .1
            .get_buffer(LightsData::ATMOSPHERE_PARAMS_BUFFER_IDX)
            .unwrap()
    ) {
        Some(buffer) => buffer,
        None => return
    };
    atmosphere_buffer.buffer.write(
        &render_instance,
        bytemuck::cast_slice(&[*atmosphere_uniform]),
        0
    );
}
