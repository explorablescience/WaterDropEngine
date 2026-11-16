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

use crate::prelude::{DirectionalLight, PbrDebugView, PbrSettings};

/// Resolution of the shadow map of each cascade.
pub const SHADOW_MAP_SIZE: u32 = 4096;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowParamsUniform {
    pub view_proj: [[f32; 4]; 4],
    /// xyz = light direction, w = size of a shadow map texel in world units
    pub light_dir: [f32; 4],
    pub kernel_half_size: f32,
    pub kernel_scale: f32,
    pub shadow_intensity: f32,
    /// Depth of the light frustum in world units
    pub depth_range: f32
}
impl Default for ShadowParamsUniform {
    fn default() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            light_dir: [0.0, -1.0, 0.0, 1.0],
            kernel_half_size: 1.0,
            kernel_scale: 1.0,
            shadow_intensity: 1.0,
            depth_range: 1.0
        }
    }
}

/// Parameters shared by the cascades.
#[repr(C)]
#[derive(Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowCascadesUniform {
    /// xyz = view depth where each cascade ends, w = blend fraction
    pub splits: [f32; 4],
    /// x = normal bias in texels
    pub params: [f32; 4]
}

/// Cascaded shadow mapping settings of the directional light.
///
/// Each cascade covers a range of view depths, and its shadow map covers the bounding sphere of that slice of the view
/// frustum: its size does not change when the camera rotates, and it moves by whole texels, so the shadows don't shimmer.
#[derive(Resource, Reflect, Clone, ExtractResource)]
#[reflect(Resource)]
pub struct CascadedShadowSettings {
    /// View depth where each cascade ends. The shadows fade out at the end of the last one.
    pub split_distances: [f32; 3],
    /// Fraction of each cascade, at its end, cross-faded with the next one.
    pub blend_fraction: f32,
    /// Distance towards the light, beyond the slice of a cascade, from which objects cast shadows into it.
    pub caster_distance: f32,
    /// Offset of the shadow lookups along the surface normal, in texels, against shadow acne.
    pub normal_bias: f32,

    /// PCF kernel half size in texels (0 for a single bilinear tap, 1 for 3x3 taps, ...).
    pub kernel_half_size: [u32; 3],
    /// Spacing of the PCF taps, in texels.
    pub kernel_scale: [f32; 3],
    /// Darkness of the shadows, in [0, 1].
    pub shadow_intensity: [f32; 3],

    /// Enable individual cascades for debugging
    pub cascade_enabled: [bool; 3],
    /// Draw the bounding sphere and the light box of each cascade (with the `wde` debug gizmos).
    pub show_bounds: bool
}
impl Default for CascadedShadowSettings {
    fn default() -> Self {
        Self {
            split_distances: [50.0, 150.0, 400.0],
            blend_fraction: 0.15,
            caster_distance: 200.0,
            normal_bias: 1.0,
            kernel_half_size: [3, 2, 1],
            kernel_scale: [1.0, 1.0, 1.0],
            shadow_intensity: [0.8, 0.8, 0.8],
            cascade_enabled: [true, true, true],
            show_bounds: false
        }
    }
}

/// Debug color of each cascade, in sRGB. Same as `SHADOW_DEBUG_CASCADE_COLORS` in the shaders.
pub const SHADOW_CASCADE_COLORS: [[f32; 3]; 3] =
    [[1.0, 0.3, 0.3], [0.3, 1.0, 0.3], [0.35, 0.5, 1.0]];

/// Shadow map of a cascade, as placed for a camera.
#[derive(Clone, Copy, Debug)]
pub struct ShadowCascade {
    /// View depths of the slice of the view frustum covered, including the blend with the previous cascade.
    pub near: f32,
    pub far: f32,
    /// Bounding sphere of the slice, snapped to the texels.
    pub center: Vec3,
    pub radius: f32,
    /// Size of a shadow map texel in world units.
    pub texel: f32,
    /// Depth of the light frustum in world units.
    pub depth_range: f32,
    pub view: Mat4,
    pub proj: Mat4
}
impl ShadowCascade {
    /// Box rendered into the shadow map, as the transform of a unit cube.
    pub fn light_box(&self) -> Transform {
        let light = Transform::from_matrix(self.view.inverse());
        light.mul_transform(
            Transform::from_xyz(0.0, 0.0, -self.depth_range * 0.5).with_scale(Vec3::new(
                2.0 * self.radius,
                2.0 * self.radius,
                self.depth_range
            ))
        )
    }
}

impl CascadedShadowSettings {
    /// Shadow maps of the cascades for a camera with the `aspect` ratio, and a light shining along `light_direction`.
    pub fn cascades(
        &self,
        camera_transform: &GlobalTransform,
        camera_view: &CameraView,
        aspect: f32,
        light_direction: Vec3
    ) -> [ShadowCascade; 3] {
        let dir = light_direction.normalize();
        let up = if dir.y.abs() < 0.99 { Vec3::Y } else { Vec3::X };
        let light_rotation = glam::camera::rh::view::look_at_mat4(Vec3::ZERO, dir, up);
        let tan_y = (camera_view.fov.to_radians() * 0.5).tan();
        let aspect = aspect.max(1e-3);
        let k = tan_y * (1.0 + aspect * aspect).sqrt();

        std::array::from_fn(|cascade| {
            let far = self.split_distances[cascade];
            let near = match cascade {
                0 => camera_view.znear,
                _ => self.split_distances[cascade - 1] * (1.0 - self.blend_fraction)
            };
            let far = far.max(near + 0.01);
            let (center_distance, radius) = frustum_slice_sphere(near, far, k);
            let radius = (radius * 2.0).ceil() * 0.5;
            let texel = 2.0 * radius / SHADOW_MAP_SIZE as f32;

            // Snap the center to the texels in light space
            let center =
                camera_transform.translation() + camera_transform.forward() * center_distance;
            let center_ls = light_rotation.transform_point3(center);
            let snapped = ((center_ls.truncate() / texel).floor() * texel).extend(center_ls.z);
            let center = light_rotation.inverse().transform_point3(snapped);

            let depth_range = 2.0 * radius + self.caster_distance;
            let eye = center - dir * (radius + self.caster_distance);
            ShadowCascade {
                near,
                far,
                center,
                radius,
                texel,
                depth_range,
                view: glam::camera::rh::view::look_at_mat4(eye, eye + dir, up),
                proj: ortho_rh_wgpu(-radius, radius, -radius, radius, 0.0, depth_range)
            }
        })
    }
}

/// Debug views of [`PbrDebugView`] about the shadows.
const SHADOW_DEBUG_VIEWS: [PbrDebugView; 4] = [
    PbrDebugView::Off,
    PbrDebugView::Shadow,
    PbrDebugView::ShadowCascades,
    PbrDebugView::ShadowTexels
];
/// Same ramp as `debug_ramp` in the shaders.
const TEXEL_LEGEND: [(Color32, &str); 5] = [
    (Color32::from_rgb(230, 13, 13), "≤1/8"),
    (Color32::from_rgb(255, 115, 13), "1/4"),
    (Color32::from_rgb(230, 217, 26), "1/2"),
    (Color32::from_rgb(26, 204, 26), "1"),
    (Color32::from_rgb(51, 102, 242), "≥2")
];

fn cascade_color(cascade: usize) -> Color32 {
    let [r, g, b] = SHADOW_CASCADE_COLORS[cascade].map(|c| (c * 255.0) as u8);
    Color32::from_rgb(r, g, b)
}

pub(crate) fn params_data_ui(
    mut ui_menu: ResMut<UIMenu>,
    ctx: Res<UIContext>,
    mut cascaded_settings: ResMut<CascadedShadowSettings>,
    mut pbr: ResMut<PbrSettings>,
    lights: Query<&DirectionalLight>,
    camera: Query<(&GlobalTransform, &CameraView), With<ActiveCamera>>,
    window: Query<&Window>
) {
    UIWindow::new("Shadows")
        .open(ui_menu.clicked_mut("Render/Shadows"))
        .show(&ctx.0, |ui| {
            let mut view = pbr.debug_view;
            ui.horizontal(|ui| {
                ComboBox::from_label("Debug View")
                    .selected_text(view.label())
                    .show_ui(ui, |ui| {
                        for option in SHADOW_DEBUG_VIEWS {
                            ui.selectable_value(&mut view, option, option.label());
                        }
                    });
                ui.add(Checkbox::new(
                    &mut cascaded_settings.show_bounds,
                    "Show Bounds"
                ));
            });
            if view != pbr.debug_view {
                pbr.debug_view = view;
            }
            match pbr.debug_view {
                PbrDebugView::ShadowCascades => {
                    ui.horizontal(|ui| {
                        for i in 0..3 {
                            ui.colored_label(cascade_color(i), format!("Cascade {i}"));
                        }
                        ui.label("Striped: blend");
                    });
                }
                PbrDebugView::ShadowTexels => {
                    ui.horizontal(|ui| {
                        ui.label("Shadow texels per pixel:");
                        for (color, label) in TEXEL_LEGEND {
                            ui.colored_label(color, label);
                        }
                    });
                    ui.label("Checkered by texel when they cover several pixels.");
                }
                _ => {}
            }
            ui.separator();

            let cascades = match (lights.iter().next(), camera.single(), window.single()) {
                (Some(light), Ok((transform, camera_view)), Ok(window)) => {
                    Some(cascaded_settings.cascades(
                        transform,
                        camera_view,
                        window.width() / window.height().max(1.0),
                        light.direction
                    ))
                }
                _ => None
            };

            let settings = &mut *cascaded_settings;
            ui.columns(2, |cols| {
                cols[0].add(
                    Slider::new(&mut settings.blend_fraction, 0.0..=0.5).text("Blend Fraction")
                );
                cols[1].add(
                    Slider::new(&mut settings.caster_distance, 0.0..=1000.0)
                        .text("Caster Distance")
                        .suffix(" m")
                );
            });
            ui.add(Slider::new(&mut settings.normal_bias, 0.0..=5.0).text("Normal Bias"));

            for i in 0..3 {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.colored_label(cascade_color(i), format!("Cascade {i}"));
                    if let Some(c) = cascades.as_ref().map(|c| c[i]) {
                        ui.label(format!(
                            "{:.1} – {:.1} m, sphere {:.1} m, texel {:.1} cm",
                            c.near,
                            c.far,
                            c.radius,
                            c.texel * 100.0
                        ));
                    }
                });
                ui.columns(2, |cols| {
                    cols[0].add(Checkbox::new(&mut settings.cascade_enabled[i], "Enabled"));
                    cols[1].add(
                        Slider::new(&mut settings.split_distances[i], 1.0..=1000.0)
                            .text("Split Distance")
                            .suffix(" m")
                    );
                });
                ui.columns(2, |cols| {
                    cols[0].add(
                        Slider::new(&mut settings.kernel_half_size[i], 0..=8)
                            .text("PCF Kernel Half Size")
                            .suffix(" px")
                    );
                    cols[1].add(
                        Slider::new(&mut settings.kernel_scale[i], 0.1..=3.0)
                            .text("PCF Kernel Scale")
                    );
                });
                ui.add(
                    Slider::new(&mut settings.shadow_intensity[i], 0.0..=1.0)
                        .text("Shadow Intensity")
                );
            }
        });
}

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

/// Smallest sphere containing the view frustum between the view depths `near` and `far`, as its distance along the
/// view axis and its radius. `k` is the distance of the frustum corners to the view axis, per unit of depth.
fn frustum_slice_sphere(near: f32, far: f32, k: f32) -> (f32, f32) {
    let center = (far + near) * (1.0 + k * k) * 0.5;
    if center >= far {
        (far, k * far)
    } else {
        (center, ((far - center).powi(2) + (k * far).powi(2)).sqrt())
    }
}

pub(crate) fn extract_cascaded_shadow_params(
    lights: ExtractWorld<Query<&DirectionalLight>>,
    settings: ExtractWorld<Res<CascadedShadowSettings>>,
    camera: ExtractWorld<Single<(&GlobalTransform, &CameraView), With<ActiveCamera>>>,
    window: ExtractWorld<Query<&Window>>,
    mut params: ResMut<ExtractedCascadedShadowParams>
) {
    let Some(light) = lights.iter().next() else {
        return;
    };
    let Ok(window) = window.single() else {
        return;
    };
    let (camera_transform, camera_view) = **camera;
    let aspect = window.width() / window.height().max(1.0);
    let dir = light.direction.normalize();

    let cascades = settings.cascades(camera_transform, camera_view, aspect, dir);
    for (i, cascade) in cascades.iter().enumerate() {
        params.0[i] = ShadowParamsUniform {
            view_proj: (cascade.proj * cascade.view).to_cols_array_2d(),
            light_dir: [dir.x, dir.y, dir.z, cascade.texel],
            kernel_half_size: settings.kernel_half_size[i] as f32,
            kernel_scale: settings.kernel_scale[i],
            shadow_intensity: settings.shadow_intensity[i].clamp(0.0, 1.0),
            depth_range: cascade.depth_range
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
    buffer.buffer.write(
        &render_instance.0.read().unwrap(),
        bytemuck::cast_slice(&extracted.0),
        0
    );

    if let Some(cascades_buffer_handle) = data.get_buffer(ShadowMapData::CASCADES_BUFFER_IDX)
        && let Some(cascades_buffer) = buffers.get(&cascades_buffer_handle)
    {
        let [s0, s1, s2] = settings.split_distances;
        let cascades = ShadowCascadesUniform {
            splits: [s0, s1, s2, settings.blend_fraction.clamp(0.0, 1.0)],
            params: [settings.normal_bias.max(0.0), 0.0, 0.0, 0.0]
        };
        cascades_buffer.buffer.write(
            &render_instance.0.read().unwrap(),
            bytemuck::cast_slice(&[cascades]),
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
    pub const CASCADES_BUFFER_IDX: u32 = 4;
}
impl RenderData for ShadowMapData {
    type Params = (SQuery<&'static Window>, SRes<Messages<SurfaceResized>>);

    fn describe((_window, _): &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        let size = (SHADOW_MAP_SIZE, SHADOW_MAP_SIZE);
        builder
            .add_texture(
                Self::SHADOW_MAP_CASCADE_0,
                Texture {
                    label: "shadow-map-cascade-0".to_string(),
                    size,
                    format: TextureFormat::Depth32Float,
                    usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    compare: Some(CompareFunction::LessEqual),
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
                    compare: Some(CompareFunction::LessEqual),
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
                    compare: Some(CompareFunction::LessEqual),
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
                Self::CASCADES_BUFFER_IDX,
                Buffer {
                    label: "shadow-cascades".to_string(),
                    size: std::mem::size_of::<ShadowCascadesUniform>(),
                    usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                    content: Some(
                        bytemuck::cast_slice(&[ShadowCascadesUniform::default()]).to_vec()
                    )
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cascades() -> [ShadowCascade; 3] {
        let camera = GlobalTransform::from(
            Transform::from_xyz(10.0, 40.0, -5.0).looking_at(Vec3::new(30.0, 0.0, 20.0), Vec3::Y)
        );
        CascadedShadowSettings::default().cascades(
            &camera,
            &CameraView::default(),
            16.0 / 9.0,
            Vec3::new(0.4, -1.0, 0.3)
        )
    }

    #[test]
    fn light_box_is_the_shadow_frustum() {
        for cascade in cascades() {
            let view_proj = cascade.proj * cascade.view;
            let light_box = cascade.light_box().to_matrix();
            for corner in [-0.5f32, 0.5].into_iter().flat_map(|x| {
                [-0.5f32, 0.5]
                    .into_iter()
                    .flat_map(move |y| [-0.5f32, 0.5].map(|z| Vec3::new(x, y, z)))
            }) {
                let ndc = view_proj.project_point3(light_box.transform_point3(corner));
                let expected = Vec3::new(corner.x * 2.0, corner.y * 2.0, 0.5 - corner.z);
                assert!(ndc.abs_diff_eq(expected, 1e-3), "{ndc} != {expected}");
            }
        }
    }

    #[test]
    fn sphere_is_inside_the_shadow_map() {
        for cascade in cascades() {
            let ndc = (cascade.proj * cascade.view).project_point3(cascade.center);
            assert!(ndc.truncate().abs_diff_eq(Vec2::ZERO, 1e-3));
            assert!(ndc.z > 0.0 && ndc.z < 1.0);
        }
    }
}
