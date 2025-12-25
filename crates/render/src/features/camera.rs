use bevy::prelude::*;
use wde_wgpu::{bind_group::{BindGroup, BindGroupLayout, WgpuBindGroup, WgpuBindGroupLayout}, buffer::{BufferBindingType, BufferUsage}, instance::WRenderInstance, render_pipeline::WShaderStages};

use crate::{assets::{Buffer, GpuBuffer, RenderAssets}, components::{ActiveCamera, CameraUniform, CameraView}, core::{extract_macros::ExtractWorld, Extract, Render, RenderApp, RenderSet}};

/// Struct to hold the camera uniform layout description.
#[derive(Resource)]
pub struct CameraFeatureRender {
    pub layout: BindGroupLayout,
    pub layout_built: WgpuBindGroupLayout,
    pub bind_group: Option<WgpuBindGroup>,
}
impl FromWorld for CameraFeatureRender {
    fn from_world(world: &mut World) -> Self {
        let render_instance = world.get_resource::<WRenderInstance<'static>>().unwrap();

        // Create the camera layout
        let layout = BindGroupLayout::new("camera", |builder| {
            builder.add_buffer(
                0, WShaderStages::VERTEX | WShaderStages::FRAGMENT,
                BufferBindingType::Uniform);
        });
        let layout_built = layout.build(&render_instance.data.read().unwrap());
        
        CameraFeatureRender { layout, layout_built, bind_group: None }
    }
}

/// Struct to hold the camera uniform buffer.
#[derive(Resource, Default)]
pub struct CameraFeatureBuffer {
    pub buffer: Handle<Buffer>,
}

pub struct CameraFeature;
impl Plugin for CameraFeature {
    fn build(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Extract, extract)
            .add_systems(Render, build_bind_group.in_set(RenderSet::BindGroups))
            .add_systems(Render, update_buffer.in_set(RenderSet::Prepare))
            .init_resource::<CameraFeatureRender>()
            .init_resource::<CameraUniform>();
    }

    fn finish(&self, app: &mut App) {
        // Create the camera buffer (need that the assets have been initialized)
        let buffer: Handle<Buffer> = app.world_mut().add_asset(Buffer {
            label: "camera".to_string(),
            size: std::mem::size_of::<CameraUniform>(),
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
            content: None,
        });
        
        // Add resources
        app.get_sub_app_mut(RenderApp).unwrap()
            .insert_resource(CameraFeatureBuffer { buffer });
    }
}

// Create the bind group for the camera
fn build_bind_group(
    render_instance: Res<WRenderInstance<'static>>, mut camera_feature_render: ResMut<CameraFeatureRender>,
    camera_buffer: Res<CameraFeatureBuffer>, mut buffers: ResMut<RenderAssets<GpuBuffer>>)
{
    // Check if the bind group is already created
    if camera_feature_render.bind_group.is_some() {
        return;
    }

    // Create the bind group
    if let Some(camera_buffer) = buffers.get_mut(&camera_buffer.buffer) {
        let render_instance = render_instance.data.read().unwrap();
        let bind_group = BindGroup::build("camera", &render_instance, &camera_feature_render.layout_built, &vec![
            BindGroup::buffer(0, &camera_buffer.buffer)
        ]);
        camera_feature_render.bind_group = Some(bind_group);
    }
}

// Extract the texture handle every frame
fn extract(
    (cameras, mut camera_uniform): (
        ExtractWorld<Query<(&Transform, &CameraView), With<ActiveCamera>>>, ResMut<CameraUniform>
    ), window: ExtractWorld<Query<&Window>>)
{
    if let (
        Ok((transform, view)), Ok(window)
    ) = (cameras.single(), window.single()) {
        // Update the camera uniform
        let aspect_ratio = window.width() / window.height();
        *camera_uniform = CameraUniform::new(transform, view, aspect_ratio);
    }
}

// Update the camera buffer
fn update_buffer(
    (render_instance, camera_uniform, camera_buffer): (
        Res<WRenderInstance<'static>>, Res<CameraUniform>, Res<CameraFeatureBuffer>
    ),
    mut buffers: ResMut<RenderAssets<GpuBuffer>>
) {
    // Update the camera buffer
    if let Some(camera_buffer) = buffers.get_mut(&camera_buffer.buffer) {
        let render_instance = render_instance.data.read().unwrap();
        camera_buffer.buffer.write(&render_instance, bytemuck::cast_slice(&[camera_uniform.to_owned()]), 0);
    }
}
