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

/// Cascaded shadow mapping settings with per-cascade configuration.
#[derive(Resource, Reflect, Clone, ExtractResource)]
#[reflect(Resource)]
pub struct CascadedShadowSettings {
    /// Linear depth split distances: [cascade0_end, cascade1_end, cascade2_end]
    pub split_distances: [f32; 3],
    /// Orthographic box half-width per cascade
    pub ortho_sizes: [f32; 3],
    /// Frustum depth per cascade
    pub ortho_depths: [f32; 3],
    /// Enable individual cascades for debugging
    pub cascade_enabled: [bool; 3]
}
impl Default for CascadedShadowSettings {
    fn default() -> Self {
        Self {
            split_distances: [50.0, 150.0, 400.0],
            ortho_sizes: [60.0, 200.0, 400.0],
            ortho_depths: [100.0, 300.0, 800.0],
            cascade_enabled: [true, true, true]
        }
    }
}

pub(crate) fn params_data_ui(
    mut ui_menu: ResMut<UIMenu>,
    ctx: Res<UIContext>,
    mut cascaded_settings: ResMut<CascadedShadowSettings>
) {
    UIWindow::new("Shadow Settings")
        .open(ui_menu.clicked_mut("PBR/Shadows"))
        .show(&ctx.0, |ui| {
            ui.label("Cascaded Shadow Maps:");
            ui.label("Cascade 0 (Near):");
            ui.add(
                DragValue::new(&mut cascaded_settings.split_distances[0])
                    .prefix("Split distance: ")
                    .speed(1.0)
                    .range(1.0..=500.0)
            );
            ui.add(
                DragValue::new(&mut cascaded_settings.ortho_sizes[0])
                    .prefix("Ortho size: ")
                    .speed(1.0)
            );
            ui.add(
                DragValue::new(&mut cascaded_settings.ortho_depths[0])
                    .prefix("Ortho depth: ")
                    .speed(1.0)
            );

            ui.label("Cascade 1 (Mid):");
            ui.add(
                DragValue::new(&mut cascaded_settings.split_distances[1])
                    .prefix("Split distance: ")
                    .speed(1.0)
                    .range(1.0..=500.0)
            );
            ui.add(
                DragValue::new(&mut cascaded_settings.ortho_sizes[1])
                    .prefix("Ortho size: ")
                    .speed(1.0)
            );
            ui.add(
                DragValue::new(&mut cascaded_settings.ortho_depths[1])
                    .prefix("Ortho depth: ")
                    .speed(1.0)
            );

            ui.label("Cascade 2 (Far):");
            ui.add(
                DragValue::new(&mut cascaded_settings.split_distances[2])
                    .prefix("Split distance: ")
                    .speed(1.0)
                    .range(1.0..=500.0)
            );
            ui.add(
                DragValue::new(&mut cascaded_settings.ortho_sizes[2])
                    .prefix("Ortho size: ")
                    .speed(1.0)
            );
            ui.add(
                DragValue::new(&mut cascaded_settings.ortho_depths[2])
                    .prefix("Ortho depth: ")
                    .speed(1.0)
            );

            ui.separator();
            ui.label("Debug - Cascade Visibility:");
            ui.checkbox(
                &mut cascaded_settings.cascade_enabled[0],
                "Cascade 0 (Near)"
            );
            ui.checkbox(&mut cascaded_settings.cascade_enabled[1], "Cascade 1 (Mid)");
            ui.checkbox(&mut cascaded_settings.cascade_enabled[2], "Cascade 2 (Far)");
        });
}

/// Extracted shadow params for the render world.
#[derive(Resource, Default, Clone, Copy)]
pub struct ExtractedShadowParams(pub ShadowParamsUniform);

/// Extracted cascaded shadow params for the render world (3 cascades).
#[derive(Resource, Default, Clone, Copy)]
pub struct ExtractedCascadedShadowParams(pub [ShadowParamsUniform; 3]);

/// Tracks which cascades are enabled for rendering.
#[derive(Resource, Default, Clone, Copy)]
pub struct CascadeEnabledFlags(pub [bool; 3]);

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

pub(crate) fn extract_cascaded_shadow_params(
    lights: ExtractWorld<Query<&DirectionalLight>>,
    settings: ExtractWorld<Res<CascadedShadowSettings>>,
    camera: ExtractWorld<Single<&GlobalTransform, With<ActiveCamera>>>,
    mut params: ResMut<ExtractedCascadedShadowParams>
) {
    let Some(light) = lights.iter().next() else {
        return;
    };

    let dir = light.direction.normalize();
    let camera_pos = camera.translation();
    let camera_forward = camera.forward();

    for cascade_idx in 0..3 {
        let ortho_size = settings.ortho_sizes[cascade_idx];
        let ortho_depth = settings.ortho_depths[cascade_idx];

        // Position frustum at midpoint of this cascade's depth range
        let prev_split = if cascade_idx == 0 {
            0.0
        } else {
            settings.split_distances[cascade_idx - 1]
        };
        let curr_split = settings.split_distances[cascade_idx];
        let center_distance = (prev_split + curr_split) / 2.0;
        let mut center = camera_pos + camera_forward * center_distance;

        // Fine grid quantization per cascade
        let texel_world_size = ortho_size / 2048.0;
        center = (center / texel_world_size).round() * texel_world_size;

        // Position eye along light direction
        let eye = center - dir * (ortho_depth * 0.5);
        let up = if dir.y.abs() < 0.99 { Vec3::Y } else { Vec3::X };
        let view = Mat4::look_at_rh(eye, center, up);
        let half = ortho_size * 0.5;
        let proj = ortho_rh_wgpu(-half, half, -half, half, 0.0, ortho_depth);

        params.0[cascade_idx] = ShadowParamsUniform {
            view_proj: (proj * view).to_cols_array_2d(),
            light_dir: [dir.x, dir.y, dir.z, 0.0005]
        };
    }
}

pub(crate) fn extract_cascade_enabled_flags(
    settings: ExtractWorld<Res<CascadedShadowSettings>>,
    mut flags: ResMut<CascadeEnabledFlags>
) {
    flags.0 = settings.cascade_enabled;
}

pub(crate) fn update_cascaded_shadow_params_buffer(
    shadow_data: ResRenderData<ShadowMapData>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    render_instance: Res<RenderInstance>,
    extracted: Res<ExtractedCascadedShadowParams>,
    settings: Res<CascadedShadowSettings>
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
    // Write all 3 cascades to the params buffer
    buffer.buffer.write(
        &render_instance.0.read().unwrap(),
        bytemuck::cast_slice(&extracted.0),
        0
    );

    // Update cascade splits buffer
    if let Some(splits_buffer_handle) = data.get_buffer(ShadowMapData::CASCADE_SPLITS_BUFFER_IDX)
        && let Some(splits_buffer) = buffers.get(&splits_buffer_handle)
    {
        let splits_data = [
            settings.split_distances[0],
            settings.split_distances[1],
            settings.split_distances[2],
            0.0_f32
        ];
        splits_buffer.buffer.write(
            &render_instance.0.read().unwrap(),
            bytemuck::cast_slice(&[splits_data]),
            0
        );
    }
}

#[derive(Asset, Clone, TypePath, Default)]
pub struct ShadowMapData;
impl ShadowMapData {
    pub const SHADOW_MAP_IDX: u32 = 0;
    pub const SHADOW_MAP_CASCADE_0: u32 = 0;
    pub const SHADOW_MAP_CASCADE_1: u32 = 1;
    pub const SHADOW_MAP_CASCADE_2: u32 = 2;
    pub const PARAMS_BUFFER_IDX: u32 = 3;
    pub const CASCADE_SPLITS_BUFFER_IDX: u32 = 4;
}
impl RenderData for ShadowMapData {
    type Params = (SQuery<&'static Window>, SRes<Messages<SurfaceResized>>);

    fn describe((_window, _): &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        let size = (2048, 2048);
        builder
            .add_texture(
                Self::SHADOW_MAP_CASCADE_0,
                Texture {
                    label: "shadow-map-cascade-0".to_string(),
                    size,
                    format: TextureFormat::Depth32Float,
                    usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    filterable: true,
                    ..Default::default()
                }
            )
            .add_texture(
                Self::SHADOW_MAP_CASCADE_1,
                Texture {
                    label: "shadow-map-cascade-1".to_string(),
                    size,
                    format: TextureFormat::Depth32Float,
                    usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    filterable: true,
                    ..Default::default()
                }
            )
            .add_texture(
                Self::SHADOW_MAP_CASCADE_2,
                Texture {
                    label: "shadow-map-cascade-2".to_string(),
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
                    size: std::mem::size_of::<ShadowParamsUniform>() * 3,
                    usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                    content: Some(
                        bytemuck::cast_slice(&[ShadowParamsUniform::default(); 3]).to_vec()
                    )
                }
            )
            .add_buffer(
                Self::CASCADE_SPLITS_BUFFER_IDX,
                Buffer {
                    label: "cascade-splits".to_string(),
                    size: std::mem::size_of::<[f32; 4]>(),
                    usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                    content: Some(bytemuck::cast_slice(&[[50.0_f32, 150.0, 400.0, 0.0]]).to_vec())
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
