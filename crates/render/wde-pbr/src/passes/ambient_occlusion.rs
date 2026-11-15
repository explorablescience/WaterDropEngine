use wde_logger::prelude::*;

use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SQuery, SRes, SResMut}
    },
    prelude::*
};
use bytemuck::{Pod, Zeroable};
use wde_camera::prelude::*;
use wde_renderer::{prelude::*, wgpu_utils::*};

use crate::prelude::*;

pub(crate) struct AmbientOcclusionPlugin;
impl Plugin for AmbientOcclusionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AmbientOcclusionSettings>()
            .register_type::<AmbientOcclusionSettings>()
            .add_plugins((
                RenderDataRegisterPlugin::<AmbientOcclusionData>::default(),
                RenderBindingRegisterPlugin::<AmbientOcclusionInputBinding>::default(),
                RenderBindingRegisterPlugin::<AmbientOcclusionBlurXBinding>::default(),
                RenderBindingRegisterPlugin::<AmbientOcclusionBlurYBinding>::default(),
                RenderPipelineRegisterPlugin::<AmbientOcclusionPipeline>::default(),
                RenderPipelineRegisterPlugin::<AmbientOcclusionBlurPipeline>::default()
            ));
        #[cfg(debug_assertions)]
        app.add_systems(
            Update,
            edit_settings.in_set(wde_editor::prelude::EngineUiSet)
        );

        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        render_app
            .init_resource::<AmbientOcclusionUniform>()
            .add_systems(Extract, extract)
            .add_systems(Render, update_buffer.in_set(RenderSet::Prepare));
        render_app
            .world_mut()
            .resource_mut::<RenderGraph>()
            .add_pass::<RenderPassAmbientOcclusion>();
    }
}

// ── SETTINGS ────────────────────────────────────────────────────

/// Screen-space ambient occlusion (GTAO) of the opaque geometry, applied to the ambient light only.
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct AmbientOcclusionSettings {
    pub enabled: bool,
    /// Distance in world units within which the geometry occludes.
    pub radius: f32,
    /// Fraction of the radius over which the occlusion fades out, in [0, 1].
    pub falloff: f32,
    /// Exponent of the visibility: above 1 darkens the occlusion.
    pub intensity: f32,
    /// Number of directions sampled around each pixel.
    pub slices: u32,
    /// Number of samples on each side of a direction.
    pub steps: u32,
    /// Maximum radius on the screen, in pixels, bounding the cost of close-ups.
    pub max_screen_radius: f32,
    /// Depth sensitivity of the blur and of the upsampling: higher keeps the edges sharper.
    pub blur_sharpness: f32,
    /// Shows the occlusion instead of the lit image.
    pub debug_view: bool
}
impl Default for AmbientOcclusionSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            radius: 1.5,
            falloff: 0.6,
            intensity: 1.5,
            slices: 3,
            steps: 4,
            max_screen_radius: 192.0,
            blur_sharpness: 40.0,
            debug_view: false
        }
    }
}

#[repr(C)]
#[derive(Resource, Clone, Copy, Pod, Zeroable)]
pub(crate) struct AmbientOcclusionUniform {
    /// x=radius, y=falloff, z=intensity, w=enabled
    effect: [f32; 4],
    /// x=slices, y=steps, z=max screen radius, w=blur sharpness
    quality: [f32; 4],
    /// x=debug view
    debug: [f32; 4]
}
impl Default for AmbientOcclusionUniform {
    fn default() -> Self {
        Self::from_settings(&AmbientOcclusionSettings::default())
    }
}
impl AmbientOcclusionUniform {
    fn from_settings(settings: &AmbientOcclusionSettings) -> Self {
        let flag = |b: bool| if b { 1.0 } else { 0.0 };
        Self {
            effect: [
                settings.radius.max(1e-3),
                settings.falloff.clamp(1e-3, 1.0),
                settings.intensity.max(0.0),
                flag(settings.enabled)
            ],
            quality: [
                settings.slices.max(1) as f32,
                settings.steps.max(1) as f32,
                settings.max_screen_radius.max(1.0),
                settings.blur_sharpness.max(0.0)
            ],
            debug: [flag(settings.enabled && settings.debug_view), 0.0, 0.0, 0.0]
        }
    }

    fn enabled(&self) -> bool {
        self.effect[3] > 0.0
    }
}

fn extract(
    settings: ExtractWorld<Res<AmbientOcclusionSettings>>,
    mut uniform: ResMut<AmbientOcclusionUniform>
) {
    *uniform = AmbientOcclusionUniform::from_settings(&settings);
}

fn update_buffer(
    data: ResRenderData<AmbientOcclusionData>,
    buffers: Res<RenderAssets<GpuBuffer>>,
    render_instance: Res<RenderInstance>,
    uniform: Res<AmbientOcclusionUniform>
) {
    if let Some(buffer) = data
        .iter()
        .next()
        .and_then(|(_, d)| d.get_buffer(AmbientOcclusionData::PARAMS_IDX))
        .and_then(|b| buffers.get(&b))
    {
        buffer.buffer.write(
            &render_instance.0.read().unwrap(),
            bytemuck::cast_slice(&[*uniform]),
            0
        );
    }
}

// ── DATA ────────────────────────────────────────────────────────

/// Half resolution textures of the ambient occlusion, resized with the window, and its parameters.
///
/// Each texel holds the visibility (r) and the linear depth (g) of the full resolution pixel at twice its coordinates.
#[derive(Asset, Clone, TypePath, Default)]
pub struct AmbientOcclusionData;
impl AmbientOcclusionData {
    /// Output of the GTAO, then of the blur: read by the lighting pass.
    pub const OCCLUSION_IDX: u32 = 0;
    /// Intermediate texture of the separable blur.
    pub const BLUR_IDX: u32 = 1;
    pub const PARAMS_IDX: u32 = 2;

    pub const FORMAT: TextureFormat = TextureFormat::Rg16Float;
}
impl RenderData for AmbientOcclusionData {
    type Params = (SQuery<&'static Window>, SRes<Messages<SurfaceResized>>);

    fn describe((window, _): &mut SystemParamItem<Self::Params>, builder: &mut RenderDataBuilder) {
        let size = {
            let window = window.single().unwrap();
            (
                window.resolution.physical_width().div_ceil(2).max(1),
                window.resolution.physical_height().div_ceil(2).max(1)
            )
        };
        for (idx, label) in [
            (Self::OCCLUSION_IDX, "ambient-occlusion"),
            (Self::BLUR_IDX, "ambient-occlusion-blur")
        ] {
            builder.add_texture(
                idx,
                Texture {
                    label: label.to_string(),
                    size,
                    format: Self::FORMAT,
                    usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    filterable: false,
                    ..Default::default()
                }
            );
        }
        builder.add_buffer(
            Self::PARAMS_IDX,
            Buffer {
                label: "ambient-occlusion-params".to_string(),
                size: std::mem::size_of::<AmbientOcclusionUniform>(),
                usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                content: Some(bytemuck::cast_slice(&[AmbientOcclusionUniform::default()]).to_vec())
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

/// Inputs of the GTAO: the linear depth and the parameters.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct AmbientOcclusionInputBinding;
impl RenderBinding for AmbientOcclusionInputBinding {
    type Params = (
        SRenderData<DeferredTextures>,
        SRenderData<AmbientOcclusionData>
    );

    fn describe(
        &mut self,
        (deferred, data): &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder
            .add_texture_view(deferred, DeferredTextures::DEPTH_IDX)
            .add_buffer(data, AmbientOcclusionData::PARAMS_IDX);
    }

    fn has_dependencies(&self) -> bool {
        true
    }

    fn label(&self) -> &str {
        "ambient-occlusion-input-binding"
    }
}

/// Source of the horizontal blur: the GTAO output.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct AmbientOcclusionBlurXBinding;
impl RenderBinding for AmbientOcclusionBlurXBinding {
    type Params = SRenderData<AmbientOcclusionData>;

    fn describe(
        &mut self,
        data: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder
            .add_texture_view(data, AmbientOcclusionData::OCCLUSION_IDX)
            .add_buffer(data, AmbientOcclusionData::PARAMS_IDX);
    }

    fn has_dependencies(&self) -> bool {
        true
    }

    fn label(&self) -> &str {
        "ambient-occlusion-blur-x-binding"
    }
}

/// Source of the vertical blur: the horizontal blur output.
#[derive(Asset, Clone, TypePath, Default)]
pub(crate) struct AmbientOcclusionBlurYBinding;
impl RenderBinding for AmbientOcclusionBlurYBinding {
    type Params = SRenderData<AmbientOcclusionData>;

    fn describe(
        &mut self,
        data: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder
            .add_texture_view(data, AmbientOcclusionData::BLUR_IDX)
            .add_buffer(data, AmbientOcclusionData::PARAMS_IDX);
    }

    fn has_dependencies(&self) -> bool {
        true
    }

    fn label(&self) -> &str {
        "ambient-occlusion-blur-y-binding"
    }
}

// ── PASS ────────────────────────────────────────────────────────

/// Ambient occlusion: GTAO at half resolution from the linear depth of the G-buffer, then a separable depth-aware blur.
/// The lighting pass upsamples it, and multiplies the ambient light by it.
///
/// It has a render index of 15 (after the G-buffer, so that the passes editing it are accounted for), and does nothing
/// when [`AmbientOcclusionSettings::enabled`] is false.
pub struct RenderPassAmbientOcclusion;
impl RenderPass for RenderPassAmbientOcclusion {
    type Params = ();

    fn describe(_: &SystemParamItem<Self::Params>) -> RenderPassDesc {
        RenderPassDesc::default()
    }

    fn id() -> RenderPassId {
        15
    }
    fn label() -> &'static str {
        "ambient-occlusion"
    }

    fn custom_render(world: &mut World, command_buffer: &mut CommandBuffer) -> Option<bool> {
        if !world.resource::<AmbientOcclusionUniform>().enabled() {
            return Some(true);
        }
        Some(render_ambient_occlusion(world, command_buffer))
    }
}

fn render_ambient_occlusion(world: &World, command_buffer: &mut CommandBuffer) -> bool {
    let pipeline_manager = world.resource::<PipelineManager>();
    let pipeline =
        |id: Option<CachedPipelineIndex>| match id.map(|id| pipeline_manager.get_pipeline(id)) {
            Some(CachedPipelineStatus::OkRender(p)) => Some(p),
            _ => None
        };
    let (Some(gtao), Some(blur)) = (
        pipeline(
            world
                .resource::<RenderAssets<AmbientOcclusionPipeline>>()
                .iter()
                .next()
                .map(|(_, p)| p.0)
        ),
        pipeline(
            world
                .resource::<RenderAssets<AmbientOcclusionBlurPipeline>>()
                .iter()
                .next()
                .map(|(_, p)| p.0)
        )
    ) else {
        return false;
    };

    fn bind_group<B: RenderBinding + Asset + Clone>(world: &World) -> Option<&BindGroup> {
        world
            .resource::<RenderAssets<GpuRenderBinding<B>>>()
            .iter()
            .next()
            .map(|(_, b)| &b.bind_group)
    }
    let (Some(camera), Some(input), Some(blur_x), Some(blur_y)) = (
        bind_group::<CameraBinding>(world),
        bind_group::<AmbientOcclusionInputBinding>(world),
        bind_group::<AmbientOcclusionBlurXBinding>(world),
        bind_group::<AmbientOcclusionBlurYBinding>(world)
    ) else {
        return false;
    };

    let meshes = world.resource::<RenderAssets<GpuMesh>>();
    let Some((Some(vertices), Some(indices))) = world
        .resource::<PostProcessingMesh>()
        .0
        .as_ref()
        .and_then(|m| meshes.get(m.id()))
        .map(|m| (m.vertex_buffer.as_ref(), m.index_buffer.as_ref()))
    else {
        return false;
    };

    let gpu_textures = world.resource::<RenderAssets<GpuTexture>>();
    let Some((_, data)) = world
        .resource::<RenderAssets<GpuRenderData<AmbientOcclusionData>>>()
        .iter()
        .next()
    else {
        return false;
    };
    let texture = |idx| data.get_texture(idx).and_then(|t| gpu_textures.get(t.id()));
    let (Some(occlusion), Some(blurred)) = (
        texture(AmbientOcclusionData::OCCLUSION_IDX),
        texture(AmbientOcclusionData::BLUR_IDX)
    ) else {
        return false;
    };

    // GTAO into the occlusion texture, then blurred horizontally into the blur texture and back vertically
    let id = |idx| data.get_texture(idx).unwrap().id();
    let (occlusion_id, blurred_id) = (
        id(AmbientOcclusionData::OCCLUSION_IDX),
        id(AmbientOcclusionData::BLUR_IDX)
    );
    let draws = [
        (
            occlusion,
            occlusion_id,
            "gtao",
            gtao,
            vec![camera, input],
            None
        ),
        (
            blurred,
            blurred_id,
            "blur-x",
            blur,
            vec![blur_x],
            Some(0u32)
        ),
        (
            occlusion,
            occlusion_id,
            "blur-y",
            blur,
            vec![blur_y],
            Some(1u32)
        )
    ];
    for (target, target_id, step, pipeline, bind_groups, axis) in draws {
        let mut render_pass =
            match command_buffer.create_render_pass("ambient-occlusion", |builder| {
                builder.add_color_attachment(RenderPassColorAttachment {
                    texture: Some(&target.texture.view),
                    load: LoadOp::Clear(WdeColor::WHITE.into()),
                    store: StoreOp::Store,
                    resolve_target: None
                });
                Ok(())
            }) {
                Ok(render_pass) => render_pass,
                Err(e) => {
                    trace!("Failed to create the ambient occlusion render pass: {}.", e);
                    return false;
                }
            };
        if render_pass.set_pipeline(pipeline).is_err() {
            return false;
        }
        for (idx, bind_group) in bind_groups.into_iter().enumerate() {
            render_pass.set_bind_group(idx as u32, bind_group);
        }
        render_pass
            .set_vertex_buffer(0, vertices)
            .set_index_buffer(indices);
        if let Some(axis) = axis {
            render_pass.set_push_constants(ShaderStages::FRAGMENT, bytemuck::bytes_of(&axis));
        }
        if render_pass.draw_indexed(0..6, 0..1).is_err() {
            return false;
        }
        drop(render_pass);
        RenderGraphDebug::capture(world, command_buffer, step, &[target_id]);
    }
    true
}

// ── PIPELINES ───────────────────────────────────────────────────

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct AmbientOcclusionPipeline(pub CachedPipelineIndex);
impl RenderAsset for AmbientOcclusionPipeline {
    type SourceAsset = RenderPipelineAsset<AmbientOcclusionPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<CameraBinding>,
        SBinding<AmbientOcclusionInputBinding>
    );

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, camera, input): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(AmbientOcclusionPipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "ambient-occlusion",
                    vert: Some(assets_server.load("core/render/post_process/fullscreen.vert.wgsl")),
                    frag: Some(assets_server.load("core/render/pbr/ambient_occlusion.frag.wgsl")),
                    bind_group_layouts: vec![
                        camera.iter().next().map(|(_, c)| c.layout.clone()),
                        input.iter().next().map(|(_, i)| i.layout.clone()),
                    ],
                    depth: DepthDescriptor {
                        enabled: false,
                        ..default()
                    },
                    render_targets: Some(vec![AmbientOcclusionData::FORMAT]),
                    ..default()
                },
                asset
            )?
        ))
    }
}

#[derive(TypePath, Default, Clone, Debug)]
pub(crate) struct AmbientOcclusionBlurPipeline(pub CachedPipelineIndex);
impl RenderAsset for AmbientOcclusionBlurPipeline {
    type SourceAsset = RenderPipelineAsset<AmbientOcclusionBlurPipeline>;
    type Params = (
        SRes<AssetServer>,
        SResMut<PipelineManager>,
        SBinding<AmbientOcclusionBlurXBinding>
    );

    fn prepare(
        _id: AssetId<Self::SourceAsset>,
        asset: Self::SourceAsset,
        (assets_server, pipeline_manager, source): &mut SystemParamItem<Self::Params>
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(AmbientOcclusionBlurPipeline(
            pipeline_manager.create_render_pipeline(
                RenderPipelineDescriptor {
                    label: "ambient-occlusion-blur",
                    vert: Some(assets_server.load("core/render/post_process/fullscreen.vert.wgsl")),
                    frag: Some(
                        assets_server.load("core/render/pbr/ambient_occlusion_blur.frag.wgsl")
                    ),
                    bind_group_layouts: vec![source.iter().next().map(|(_, s)| s.layout.clone())],
                    push_constants: vec![PushConstantDescriptor {
                        stages: ShaderStages::FRAGMENT,
                        offset: 0,
                        size: 4
                    }],
                    depth: DepthDescriptor {
                        enabled: false,
                        ..default()
                    },
                    render_targets: Some(vec![AmbientOcclusionData::FORMAT]),
                    ..default()
                },
                asset
            )?
        ))
    }
}

// ── EDITOR ──────────────────────────────────────────────────────

#[cfg(debug_assertions)]
fn edit_settings(
    ctx: Res<wde_editor::prelude::UIContext>,
    mut settings: ResMut<AmbientOcclusionSettings>,
    mut ui_menu: ResMut<wde_editor::prelude::UIMenu>
) {
    use wde_editor::prelude::*;

    UIWindow::new("Ambient Occlusion")
        .resizable(false)
        .default_pos([100.0, 820.0])
        .open(ui_menu.clicked_mut("PBR/Ambient Occlusion"))
        .show(&ctx.0, |ui| {
            let s = &mut *settings;
            ui.columns(2, |cols| {
                cols[0].add(Checkbox::new(&mut s.enabled, "Enabled"));
                cols[1].add(Checkbox::new(&mut s.debug_view, "Debug View"));
            });
            ui.columns(2, |cols| {
                cols[0].add(
                    Slider::new(&mut s.radius, 0.05..=20.0)
                        .logarithmic(true)
                        .text("Radius")
                        .suffix(" m")
                );
                cols[1].add(Slider::new(&mut s.falloff, 0.0..=1.0).text("Falloff"));
            });
            ui.columns(2, |cols| {
                cols[0].add(Slider::new(&mut s.intensity, 0.0..=4.0).text("Intensity"));
                cols[1].add(Slider::new(&mut s.blur_sharpness, 0.0..=200.0).text("Blur Sharpness"));
            });
            ui.columns(2, |cols| {
                cols[0].add(Slider::new(&mut s.slices, 1..=8).text("Slices"));
                cols[1].add(Slider::new(&mut s.steps, 1..=16).text("Steps"));
            });
            ui.add(
                Slider::new(&mut s.max_screen_radius, 8.0..=512.0)
                    .text("Max Screen Radius")
                    .suffix(" px")
            );
        });
}
