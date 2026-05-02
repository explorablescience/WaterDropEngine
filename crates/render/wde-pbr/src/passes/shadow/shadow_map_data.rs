use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SQuery, SRes}
    },
    prelude::*
};
use wde_camera::prelude::*;
use wde_editor::prelude::*;
use wde_renderer::prelude::*;

use crate::prelude::DirectionalLight;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowParamsUniform {
    pub view_proj: [[f32; 4]; 4],
    pub light_dir: [f32; 4] // xyz = direction toward sun, w = depth bias
}
impl Default for ShadowParamsUniform {
    fn default() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            light_dir: [0.0, 1.0, 0.0, 0.005]
        }
    }
}

/// Settings for shadow rendering, defined in the main world.
#[derive(Resource, Reflect, Clone)]
#[reflect(Resource)]
pub struct ShadowSettings {
    /// Half-width of the orthographic shadow box (covers scene_center ± ortho_size/2).
    pub ortho_size: f32,
    /// Total depth of the shadow frustum (near=0, far=ortho_depth).
    pub ortho_depth: f32,
    /// Depth bias added when comparing shadow depth to avoid self-shadowing.
    pub depth_bias: f32
}
impl Default for ShadowSettings {
    fn default() -> Self {
        Self {
            ortho_size: 100.0,
            ortho_depth: 200.0,
            depth_bias: 0.0001
        }
    }
}

pub(crate) fn params_data_ui(
    mut ui_menu: ResMut<UIMenu>,
    ctx: Res<UIContext>,
    mut settings: ResMut<ShadowSettings>
) {
    UIWindow::new("Shadow Settings")
        .open(ui_menu.clicked_mut("PBR/Shadows"))
        .show(&ctx.0, |ui| {
            ui.label("Scene center:");
            ui.add(
                DragValue::new(&mut settings.ortho_size)
                    .prefix("Ortho size: ")
                    .speed(0.1)
            );
            ui.add(
                DragValue::new(&mut settings.ortho_depth)
                    .prefix("Ortho depth: ")
                    .speed(0.1)
            );
            ui.add(
                DragValue::new(&mut settings.depth_bias)
                    .prefix("Depth bias: ")
                    .speed(0.0001)
            );
        });
}

/// Extracted shadow params for the render world.
#[derive(Resource, Default, Clone, Copy)]
pub struct ExtractedShadowParams(pub ShadowParamsUniform);

/// Right-handed orthographic projection mapping Z to [0, 1] (wgpu/Vulkan NDC convention).
fn ortho_rh_wgpu(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Mat4 {
    let inv_width = 1.0 / (right - left);
    let inv_height = 1.0 / (top - bottom);
    let inv_depth = 1.0 / (near - far);
    Mat4::from_cols(
        Vec4::new(2.0 * inv_width, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 2.0 * inv_height, 0.0, 0.0),
        Vec4::new(0.0, 0.0, inv_depth, 0.0),
        Vec4::new(
            -(right + left) * inv_width,
            -(top + bottom) * inv_height,
            near * inv_depth,
            1.0
        )
    )
}

pub(crate) fn extract_shadow_params(
    lights: ExtractWorld<Query<&DirectionalLight>>,
    settings: ExtractWorld<Res<ShadowSettings>>,
    camera: ExtractWorld<Single<&GlobalTransform, With<ActiveCamera>>>,
    mut params: ResMut<ExtractedShadowParams>
) {
    let uniform = if let Some(light) = lights.iter().next() {
        let dir = light.direction.normalize();

        let mut center = camera.translation();
        // Fine grid quantization (each shadow texel = ~0.24 world units)
        // Reduces shimmer while maintaining near-continuous coverage
        let texel_world_size = settings.ortho_size / 2048.0;
        center = (center / texel_world_size).round() * texel_world_size;

        let eye = center - dir * (settings.ortho_depth * 0.5);
        let up = if dir.y.abs() < 0.99 { Vec3::Y } else { Vec3::X };
        let view = Mat4::look_at_rh(eye, center, up);
        let half = settings.ortho_size * 0.5;
        let proj = ortho_rh_wgpu(-half, half, -half, half, 0.0, settings.ortho_depth);
        ShadowParamsUniform {
            view_proj: (proj * view).to_cols_array_2d(),
            light_dir: [dir.x, dir.y, dir.z, settings.depth_bias]
        }
    } else {
        ShadowParamsUniform::default()
    };
    params.0 = uniform;
}

pub(crate) fn update_shadow_params_buffer(
    shadow_data: ResRenderData<ShadowMapData>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    render_instance: Res<RenderInstance>,
    extracted: Res<ExtractedShadowParams>
) {
    let Some((_, data)) = shadow_data.iter().next() else {
        return;
    };
    let Some(buffer_handle) = data.get_buffer(ShadowMapData::PARAMS_BUFFER_IDX) else {
        return;
    };
    let Some(buffer) = buffers.get(&buffer_handle) else {
        return;
    };
    buffer.buffer.write(
        &render_instance.0.read().unwrap(),
        bytemuck::bytes_of(&extracted.0),
        0
    );
}

#[derive(Asset, Clone, TypePath, Default)]
pub struct ShadowMapData;
impl ShadowMapData {
    pub const SHADOW_MAP_IDX: u32 = 0;
    pub const PARAMS_BUFFER_IDX: u32 = 1;
}
impl RenderData for ShadowMapData {
    type Params = (SQuery<&'static Window>, SRes<Messages<SurfaceResized>>);

    fn describe((_window, _): &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        let size = (2048, 2048);
        builder
            .add_texture(
                Self::SHADOW_MAP_IDX,
                Texture {
                    label: "shadow-map".to_string(),
                    size,
                    format: TextureFormat::Depth32Float,
                    usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    filterable: true,
                    ..Default::default()
                }
            )
            .add_buffer(
                Self::PARAMS_BUFFER_IDX,
                Buffer {
                    label: "shadow-params".to_string(),
                    size: std::mem::size_of::<ShadowParamsUniform>(),
                    usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                    content: Some(bytemuck::cast_slice(&[ShadowParamsUniform::default()]).to_vec())
                }
            );
    }

    fn recreate((_, surface_resized): &SystemParamItem<Self::Params>) -> Option<bool> {
        Some(
            surface_resized
                .get_cursor()
                .read(surface_resized)
                .next()
                .is_some()
        )
    }
}
