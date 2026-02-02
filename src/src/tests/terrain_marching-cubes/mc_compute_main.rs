use bevy::prelude::*;
use wde_render::{assets::Buffer, core::{extract_macros::ExtractWorld, DeviceLimits}};
use wde_wgpu::buffer::BufferUsage;

use super::{MC_MAX_POINTS, MC_MAX_TRIANGLES};

/**
 * Description of the noise to generate the terrain.
 */
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug, Resource)]
pub struct MCTerrainNoiseParameters {
    amplitude:      f32,   // Amplitude of the noise
    frequency:      f32,   // Frequency of the noise
    ground_percent: f32,   // Percentage of the ground
    octaves:        u32,   // Number of octaves
    persistence:    f32,   // Persistence of the noise
    lacunarity:     f32    // Lacunarity of the noise
}
impl Default for MCTerrainNoiseParameters {
    fn default() -> Self {
        MCTerrainNoiseParameters {
            amplitude:      40.0,
            frequency:      0.005,
            ground_percent: 0.1,
            octaves:        8,
            persistence:    0.5,
            lacunarity:     2.0
        }
    }
}

/**
 * Description of a chunk for the compute shader used in the marching cubes algorithm.
 */
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug, Default)]
pub struct GpuMCDescription {
    pub translation:       [f32; 4], // Translation in world space of the current chunk (x, y, z, 0)
    pub chunk_length:      [f32; 4], // Length of the chunk (x, y, z, 0)
    pub chunk_sub_count:   [u32; 4], // Number of sub-chunks (x, y, z, 0)
    pub triangles_counter: u32,      // Counter of the triangles
    pub iso_level:         f32,      // Iso level
    pub padding:           [u32; 2]  // Padding
}

#[derive(Resource, Default)]
pub struct MCComputeHandler {
    // Buffers
    pub desc_cpu: Option<Handle<Buffer>>,
    pub desc_gpu: Option<Handle<Buffer>>,
    pub points_cpu: Option<Handle<Buffer>>,
    pub triangles_cpu: Option<Handle<Buffer>>,
    pub triangles_gpu: Option<Handle<Buffer>>,
    pub noise_parameters: Option<Handle<Buffer>>
}
#[derive(Resource, Default)]
pub struct MCComputeHandlerGPU {
    // Buffers
    pub desc_cpu: Option<Handle<Buffer>>,
    pub desc_gpu: Option<Handle<Buffer>>,
    pub points_cpu: Option<Handle<Buffer>>,
    pub triangles_cpu: Option<Handle<Buffer>>,
    pub triangles_gpu: Option<Handle<Buffer>>,
    pub noise_parameters: Option<Handle<Buffer>>
}
impl MCComputeHandler {
    /** Creates the asset handler buffers and instance (runs once). */
    pub fn init(
        asset_server: Res<AssetServer>,
        device_limits: Res<DeviceLimits>,
        mut commands: Commands
    ) {
        // Create the buffers
        let max_buffer_size = device_limits.0.max_storage_buffer_binding_size as usize;
        let desc_cpu = Buffer {
            label: "marching-cubes-desc-cpu".to_string(),
            size: std::mem::size_of::<GpuMCDescription>(),
            usage: BufferUsage::MAP_READ | BufferUsage::COPY_DST,
            content: None
        };
        let desc_gpu = Buffer {
            label: "marching-cubes-desc-gpu".to_string(),
            size: std::mem::size_of::<GpuMCDescription>(),
            usage: BufferUsage::STORAGE | BufferUsage::COPY_DST | BufferUsage::COPY_SRC,
            content: None
        };
        let points_cpu = Buffer {
            label: "marching-cubes-points-cpu".to_string(),
            size: std::cmp::min(std::mem::size_of::<[f32; 4]>() * MC_MAX_POINTS as usize, max_buffer_size),
            usage: BufferUsage::MAP_READ | BufferUsage::COPY_DST,
            content: None
        };
        let triangles_cpu = Buffer {
            label: "marching-cubes-triangles-cpu".to_string(),
            size: std::cmp::min(std::mem::size_of::<[f32; 12]>() * MC_MAX_TRIANGLES as usize, max_buffer_size),
            usage: BufferUsage::MAP_READ | BufferUsage::COPY_DST,
            content: None
        };
        let triangles_gpu = Buffer {
            label: "marching-cubes-triangles-gpu".to_string(),
            size: std::cmp::min(std::mem::size_of::<[f32; 12]>() * MC_MAX_TRIANGLES as usize, max_buffer_size),
            usage: BufferUsage::STORAGE | BufferUsage::COPY_SRC,
            content: None
        };
        let noise_parameters = Buffer {
            label: "marching-cubes-noise-parameters".to_string(),
            size: std::mem::size_of::<MCTerrainNoiseParameters>(),
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
            content: None
        };

        // Create the handler
        commands.insert_resource(MCComputeHandler {
            desc_cpu: Some(asset_server.add(desc_cpu)),
            desc_gpu: Some(asset_server.add(desc_gpu)),
            points_cpu: Some(asset_server.add(points_cpu)),
            triangles_cpu: Some(asset_server.add(triangles_cpu)),
            triangles_gpu: Some(asset_server.add(triangles_gpu)),
            noise_parameters: Some(asset_server.add(noise_parameters))
        });
    }


    /** Extract the new chunks in the main thread and add them to the loading chunks in the render thread. */
    pub fn extract(
        handler_update: ExtractWorld<Res<MCComputeHandler>>,
        mut handler_render: ResMut<MCComputeHandlerGPU>,
    ) {
        // Extract the buffers if they are not already extracted
        if handler_render.desc_cpu.is_none() && handler_update.desc_cpu.is_some() {
            handler_render.desc_cpu = handler_update.desc_cpu.clone();
            handler_render.desc_gpu = handler_update.desc_gpu.clone();
            handler_render.points_cpu = handler_update.points_cpu.clone();
            handler_render.triangles_cpu = handler_update.triangles_cpu.clone();
            handler_render.triangles_gpu = handler_update.triangles_gpu.clone();
            handler_render.noise_parameters = handler_update.noise_parameters.clone();
        }
    }
}
