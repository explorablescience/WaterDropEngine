//! Debug views of the render graph (see [`RenderGraphDebugView`]).
use std::{sync::Mutex, time::Duration};

use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use bytemuck::{Pod, Zeroable};
use wde_logger::trace;
use wde_wgpu::{
    bind_group::{BindGroupBuilder, BindGroupLayout},
    command_buffer::{CommandBuffer, RenderPassColorAttachment},
    render_pass::RenderPassInstance
};

use crate::{
    assets::{GpuTexture, PrepareAssetError, RenderAsset, RenderAssets, Shader, Texture},
    core::{Extract, MainWorld, Render, RenderApp, RenderInstance, RenderSet, SwapchainFrame},
    prelude::{TextureFormat, TextureSampleType, TextureUsages, WdeColor}
};

use super::{
    CachedPipelineIndex, CachedPipelineStatus, DebugFrameTimings, DebugTiming, DepthDescriptor,
    LoadOp, PipelineManager, PushConstantDescriptor, RenderPassDesc, RenderPassId,
    RenderPipelineAsset, RenderPipelineDescriptor, RenderPipelineRegisterPlugin, ShaderStages,
    StoreOp, render_graph_timings::Timings
};

pub(crate) struct RenderGraphDebugPlugin;
impl Plugin for RenderGraphDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderGraphDebugView>().add_plugins((
            RenderPipelineRegisterPlugin::<DebugViewFloatPipeline>::default(),
            RenderPipelineRegisterPlugin::<DebugViewDepthPipeline>::default(),
            RenderPipelineRegisterPlugin::<DebugViewUintPipeline>::default()
        ));
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<RenderGraphDebug>()
            .add_systems(Extract, extract)
            .add_systems(
                Render,
                (
                    begin_frame.before(RenderSet::ExtractAuto),
                    end_frame.after(RenderSet::Submit)
                )
            );
    }
}

// ── MAIN WORLD ──────────────────────────────────────────────────

/// Debug view of the render graph, in the main world: records the passes of each frame with the textures they
/// write, and draws the selected texture, as it is at the selected point of the frame, into the `target` texture.
/// The `Debug/Pipeline` window of the editor drives it.
///
/// Passes rendered by the render graph are recorded automatically, with one point after each of their sub-passes
/// showing their attachments. A pass with a [`RenderPass::custom_render`](super::RenderPass::custom_render)
/// reports its own steps with [`RenderGraphDebug::capture`].
///
/// Each pass is also timed, on the CPU while recording its commands and on the GPU with timestamps when the device
/// supports them, along with the whole frame.
#[derive(Resource, Clone)]
pub struct RenderGraphDebugView {
    /// Records the passes of each frame into the `report`, and draws the selected texture. Disable it when unused.
    pub enabled: bool,
    pub selection: Option<DebugSelection>,
    pub settings: DebugViewSettings,
    /// Texture the selected texture is drawn into, in the [`RenderGraphDebugView::TARGET_FORMAT`], with the
    /// `RENDER_ATTACHMENT` usage. It keeps its content while the selected point is not reached.
    pub target: Option<AssetId<Texture>>,
    /// Passes of the last rendered frame, in their execution order.
    pub report: Vec<DebugPassReport>,
    /// Number of frames the timings are computed over.
    pub timing_window: usize,
    pub timings: DebugFrameTimings
}
impl Default for RenderGraphDebugView {
    fn default() -> Self {
        Self {
            enabled: false,
            selection: None,
            settings: DebugViewSettings::default(),
            target: None,
            report: vec![],
            timing_window: 120,
            timings: DebugFrameTimings::default()
        }
    }
}
impl RenderGraphDebugView {
    pub const TARGET_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;
    /// Id of the last reported pass, holding the frame presented on screen after every pass.
    pub const FINAL_PASS: RenderPassId = RenderPassId::MAX;

    /// Report of the selected texture in the last frame.
    pub fn selected_texture(&self) -> Option<&DebugTextureReport> {
        let selection = self.selection?;
        self.report
            .iter()
            .find(|p| p.id == selection.pass)?
            .points
            .get(selection.point)?
            .textures
            .get(selection.texture)
    }
}

/// A texture at a point of a pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DebugSelection {
    pub pass: RenderPassId,
    /// Index of the point in [`DebugPassReport::points`].
    pub point: usize,
    /// Index of the texture in [`DebugPointReport::textures`].
    pub texture: usize
}

/// How the selected texture is displayed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DebugViewSettings {
    pub mode: DebugViewMode,
    pub channels: DebugChannels,
    /// Values mapped to black and white.
    pub range: [f32; 2],
    /// Exposure in EV stops of the [`DebugViewMode::Color`].
    pub exposure: f32,
    /// Reinhard tonemapping of the [`DebugViewMode::Color`], for HDR colors.
    pub tonemap: bool,
    /// Views the stencil aspect of depth-stencil textures instead of their depth.
    pub stencil: bool,
    pub layer: u32,
    pub mip: u32
}
impl Default for DebugViewSettings {
    fn default() -> Self {
        Self {
            mode: DebugViewMode::Color,
            channels: DebugChannels::Rgb,
            range: [0.0, 1.0],
            exposure: 0.0,
            tonemap: false,
            stencil: false,
            layer: 0,
            mip: 0
        }
    }
}
impl DebugViewSettings {
    /// Sensible settings to view a texture of this format.
    pub fn for_format(format: TextureFormat) -> Self {
        let data = |channels, range| Self {
            mode: DebugViewMode::Data,
            channels,
            range,
            ..Default::default()
        };
        match format {
            // Perspective depth: the scene is close to the far plane
            TextureFormat::Depth24Plus | TextureFormat::Depth24PlusStencil8 => {
                data(DebugChannels::R, [0.99, 1.0])
            }
            f if f.has_depth_aspect() => data(DebugChannels::R, [0.0, 1.0]),
            TextureFormat::Stencil8 => Self::for_stencil(),
            TextureFormat::R32Float => data(DebugChannels::R, [0.0, 100.0]),
            f if f.sample_type(None, None) == Some(TextureSampleType::Uint) => {
                data(DebugChannels::Rgb, [0.0, 255.0])
            }
            f if f.is_srgb() => Self::default(),
            f if f.components() <= 2 => data(DebugChannels::Rgb, [0.0, 1.0]),
            TextureFormat::Rgba16Float
            | TextureFormat::Rgba32Float
            | TextureFormat::Rg11b10Ufloat => Self {
                tonemap: true,
                ..Default::default()
            },
            _ => data(DebugChannels::Rgb, [0.0, 1.0])
        }
    }

    /// Settings to view the stencil aspect of a texture.
    pub fn for_stencil() -> Self {
        Self {
            mode: DebugViewMode::Integer,
            channels: DebugChannels::R,
            range: [0.0, 255.0],
            stencil: true,
            ..Default::default()
        }
    }
}

/// Linear color of an integer value in the [`DebugViewMode::Integer`]: black for 0, and hues spread by the golden
/// ratio for the others.
pub fn debug_integer_color(value: u32) -> [f32; 3] {
    if value == 0 {
        return [0.0; 3];
    }
    let hue = ((value - 1) as f32 * 0.618_034).fract();
    [0.0, 2.0 / 3.0, 1.0 / 3.0].map(|offset| {
        let ramp = (((hue + offset).fract() * 6.0 - 3.0).abs() - 1.0).clamp(0.0, 1.0);
        1.0 + (ramp - 1.0) * DEBUG_INTEGER_SATURATION
    })
}
const DEBUG_INTEGER_SATURATION: f32 = 0.8;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DebugViewMode {
    /// Values shown as they are, from black to white.
    Data,
    /// Linear colors, with an exposure and an optional tonemapping.
    #[default]
    Color,
    /// Rounded values, each with its own color (see [`debug_integer_color`]), such as stencil values. The `Rgb`
    /// channels show the red one.
    Integer
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DebugChannels {
    #[default]
    Rgb,
    R,
    G,
    B,
    A
}

/// A pass of the last frame.
#[derive(Clone, Debug)]
pub struct DebugPassReport {
    pub id: RenderPassId,
    pub label: &'static str,
    /// Rendered by [`RenderPass::custom_render`](super::RenderPass::custom_render), its points being the steps it reported.
    pub custom: bool,
    /// Points of the pass where its textures can be viewed: after each sub-pass, or each step of a custom pass.
    pub points: Vec<DebugPointReport>,
    /// Time to record the commands of the pass, its sub-passes described.
    pub cpu_time: Option<DebugTiming>,
    /// Time of the pass on the GPU, including the debug view drawn in it.
    pub gpu_time: Option<DebugTiming>
}

#[derive(Clone, Debug)]
pub struct DebugPointReport {
    pub label: String,
    pub textures: Vec<DebugTextureReport>
}

#[derive(Clone, Debug)]
pub struct DebugTextureReport {
    pub label: String,
    /// `None` for the swapchain, which is copied to be viewed.
    pub id: Option<AssetId<Texture>>,
    pub format: Option<TextureFormat>,
    pub size: (u32, u32),
    pub layers: u32,
    pub mips: u32,
    /// Whether the texture can be drawn: the signed integer formats can't, nor the swapchain without the
    /// `COPY_SRC` usage (only set in debug builds).
    pub viewable: bool
}
impl DebugTextureReport {
    fn new(world: &World, id: Option<AssetId<Texture>>) -> Self {
        let Some(id) = id else {
            let texture = world
                .resource::<SwapchainFrame>()
                .data
                .as_ref()
                .map(|f| &f.texture.texture);
            return Self {
                label: "swapchain".to_string(),
                id: None,
                format: texture.map(|t| t.format()),
                size: texture.map_or((0, 0), |t| (t.width(), t.height())),
                layers: 1,
                mips: 1,
                viewable: texture.is_some_and(|t| t.usage().contains(TextureUsages::COPY_SRC))
            };
        };
        let Some(texture) = world.resource::<RenderAssets<GpuTexture>>().get(id) else {
            return Self {
                label: "(not ready)".to_string(),
                id: Some(id),
                format: None,
                size: (0, 0),
                layers: 0,
                mips: 0,
                viewable: false
            };
        };
        let texture = &texture.texture;
        Self {
            label: texture.label.clone(),
            id: Some(id),
            format: Some(texture.format),
            size: texture.size,
            layers: texture.layer_count,
            mips: texture.mip_level_count,
            viewable: texture.sample_count == 1
                && view_pipeline_kind(texture.sample_type(false)).is_some()
        }
    }
}

// ── RENDER WORLD ────────────────────────────────────────────────

/// Debug view of the render graph, in the render world (see [`RenderGraphDebugView`]).
#[derive(Resource, Default)]
pub struct RenderGraphDebug(Mutex<DebugState>);

#[derive(Default)]
struct DebugState {
    enabled: bool,
    selection: Option<DebugSelection>,
    settings: DebugViewSettings,
    target: Option<AssetId<Texture>>,
    /// Passes of the frame being rendered.
    report: Vec<DebugPassReport>,
    /// Passes of the last rendered frame, sent to the main world.
    finished: Vec<DebugPassReport>,
    /// Copy of the swapchain texture, which can't be read by a shader.
    swapchain_copy: Option<wde_wgpu::texture::Texture>,
    timings: Timings,
    /// Timings of the last rendered frame, sent to the main world.
    finished_timings: Option<DebugFrameTimings>
}

fn extract(mut main_world: ResMut<MainWorld>, debug: Res<RenderGraphDebug>) {
    let mut view = main_world.resource_mut::<RenderGraphDebugView>();
    let mut state = debug.0.lock().unwrap();
    state.enabled = view.enabled;
    state.selection = view.selection;
    state.settings = view.settings;
    state.target = view.target;
    if !state.enabled {
        state.timings = Timings::default();
    }
    state.timings.window = view.timing_window;
    if !state.finished.is_empty() {
        view.report = std::mem::take(&mut state.finished);
    }
    if let Some(timings) = state.finished_timings.take() {
        view.timings = timings;
    }
}

fn begin_frame(debug: Res<RenderGraphDebug>, instance: Res<RenderInstance>) {
    let mut state = debug.0.lock().unwrap();
    if state.enabled {
        state.timings.begin_frame(&instance.0.read().unwrap());
    }
}

fn end_frame(debug: Res<RenderGraphDebug>, instance: Res<RenderInstance>) {
    let mut state = debug.0.lock().unwrap();
    if !state.enabled {
        return;
    }
    state.timings.end_frame();
    let present_mode = instance
        .0
        .read()
        .unwrap()
        .surface_config
        .as_ref()
        .map(|c| c.present_mode);
    state.finished_timings = Some(state.timings.summary(present_mode));
}

impl RenderGraphDebug {
    /// Reports a step of a custom pass, that wrote the `textures`, and draws the selected one if this step is
    /// selected. Call it from [`RenderPass::custom_render`](super::RenderPass::custom_render) after each step,
    /// once its render or compute pass is ended.
    pub fn capture(
        world: &World,
        command_buffer: &mut CommandBuffer,
        label: &str,
        textures: &[AssetId<Texture>]
    ) {
        let Some(debug) = world.get_resource::<RenderGraphDebug>() else {
            return;
        };
        let mut state = debug.0.lock().unwrap();
        if !state.enabled {
            return;
        }
        let reports = textures
            .iter()
            .map(|id| DebugTextureReport::new(world, Some(*id)))
            .collect();
        let Some(pass) = state.report.last_mut() else {
            return;
        };
        pass.points.push(DebugPointReport {
            label: label.to_string(),
            textures: reports
        });
        let point = (pass.id, pass.points.len() - 1);
        if let Some(selection) = state.selection
            && (selection.pass, selection.point) == point
            && let Some(id) = textures.get(selection.texture)
        {
            draw_source(world, command_buffer, &mut state, DebugSource::Texture(*id));
        }
    }

    pub(crate) fn begin_pass(world: &World, id: RenderPassId, label: &'static str) {
        let mut state = world.resource::<RenderGraphDebug>().0.lock().unwrap();
        if state.enabled {
            state.report.push(DebugPassReport {
                id,
                label,
                custom: false,
                points: vec![],
                cpu_time: None,
                gpu_time: None
            });
        }
    }

    pub(crate) fn begin_graph(world: &World) {
        let mut state = world.resource::<RenderGraphDebug>().0.lock().unwrap();
        if state.enabled {
            state.timings.begin_graph();
        }
    }

    pub(crate) fn begin_pass_timing(
        world: &World,
        command_buffer: &mut CommandBuffer,
        id: RenderPassId
    ) {
        let mut state = world.resource::<RenderGraphDebug>().0.lock().unwrap();
        if state.enabled {
            state.timings.begin_pass(command_buffer, id);
        }
    }

    pub(crate) fn end_pass_timing(world: &World, command_buffer: &mut CommandBuffer) {
        let mut state = world.resource::<RenderGraphDebug>().0.lock().unwrap();
        if state.enabled {
            state.timings.end_pass(command_buffer);
        }
    }

    pub(crate) fn end_graph(world: &World, command_buffer: &mut CommandBuffer) {
        let mut state = world.resource::<RenderGraphDebug>().0.lock().unwrap();
        if state.enabled {
            let instance = world.resource::<RenderInstance>().0.read().unwrap();
            state.timings.end_graph(&instance, command_buffer);
        }
    }

    /// Reads back the GPU timings, once the render graph commands are submitted.
    pub(crate) fn submitted(world: &World) {
        let mut state = world.resource::<RenderGraphDebug>().0.lock().unwrap();
        if state.enabled {
            state.timings.submitted();
        }
    }

    /// Time blocked acquiring the swapchain texture.
    pub(crate) fn add_swapchain_wait(&self, wait: Duration) {
        let mut state = self.0.lock().unwrap();
        if state.enabled {
            state.timings.add_swapchain_wait(wait);
        }
    }

    /// A frame was presented, after blocking for `wait`.
    pub(crate) fn presented(&self, wait: Duration) {
        let mut state = self.0.lock().unwrap();
        if state.enabled {
            state.timings.presented(wait);
        }
    }

    pub(crate) fn end_custom_pass(world: &World) {
        let mut state = world.resource::<RenderGraphDebug>().0.lock().unwrap();
        if let Some(pass) = state.report.last_mut() {
            pass.custom = true;
        }
    }

    /// Reports the points of a pass rendered by the render graph: after each of its sub-passes, showing its
    /// attachments. Returns the selected point and its texture, if any.
    pub(crate) fn graph_pass(
        world: &World,
        desc: &RenderPassDesc,
        sub_passes: &[&str]
    ) -> Option<(usize, Option<DebugSource>)> {
        let mut state = world.resource::<RenderGraphDebug>().0.lock().unwrap();
        if !state.enabled {
            return None;
        }
        let attachments: Vec<Option<AssetId<Texture>>> = match &desc.attachments_colors {
            Some(colors) => colors.iter().map(|c| c.texture).collect(),
            None => vec![None]
        }
        .into_iter()
        .chain(desc.attachments_depth.as_ref().map(|d| d.texture))
        .collect();
        let textures: Vec<_> = attachments
            .iter()
            .map(|id| DebugTextureReport::new(world, *id))
            .collect();
        let labels = if sub_passes.is_empty() {
            &["(clear)"][..]
        } else {
            sub_passes
        };

        let selection = state.selection;
        let pass = state.report.last_mut()?;
        pass.points = labels
            .iter()
            .map(|label| DebugPointReport {
                label: label.to_string(),
                textures: textures.clone()
            })
            .collect();
        let selection = selection.filter(|s| s.pass == pass.id && s.point < pass.points.len())?;
        Some((
            selection.point,
            attachments
                .get(selection.texture)
                .map(|id| id.map_or(DebugSource::Swapchain, DebugSource::Texture))
        ))
    }

    /// Draws the selected texture of a pass rendered by the render graph.
    pub(crate) fn draw_selected(
        world: &World,
        command_buffer: &mut CommandBuffer,
        source: DebugSource
    ) {
        let mut state = world.resource::<RenderGraphDebug>().0.lock().unwrap();
        draw_source(world, command_buffer, &mut state, source);
    }

    /// Reports the [`RenderGraphDebugView::FINAL_PASS`], after every pass, and draws the presented frame if it is
    /// selected.
    pub(crate) fn final_pass(world: &World, command_buffer: &mut CommandBuffer) {
        let mut state = world.resource::<RenderGraphDebug>().0.lock().unwrap();
        if !state.enabled {
            return;
        }
        state.report.push(DebugPassReport {
            id: RenderGraphDebugView::FINAL_PASS,
            label: "final",
            custom: false,
            points: vec![DebugPointReport {
                label: "presented frame".to_string(),
                textures: vec![DebugTextureReport::new(world, None)]
            }],
            cpu_time: None,
            gpu_time: None
        });
        if state
            .selection
            .is_some_and(|s| s.pass == RenderGraphDebugView::FINAL_PASS)
        {
            draw_source(world, command_buffer, &mut state, DebugSource::Swapchain);
        }
    }

    pub(crate) fn end_frame(world: &World) {
        let mut state = world.resource::<RenderGraphDebug>().0.lock().unwrap();
        if state.enabled {
            let DebugState {
                report, timings, ..
            } = &mut *state;
            timings.fill_reports(report);
            state.finished = std::mem::take(&mut state.report);
        } else {
            state.report.clear();
        }
    }
}

// ── DRAW ────────────────────────────────────────────────────────

/// A texture drawn by the debug view.
#[derive(Clone, Copy)]
pub(crate) enum DebugSource {
    Texture(AssetId<Texture>),
    Swapchain
}

fn draw_source(
    world: &World,
    command_buffer: &mut CommandBuffer,
    state: &mut DebugState,
    source: DebugSource
) -> bool {
    match source {
        DebugSource::Texture(id) => {
            let Some(texture) = world.resource::<RenderAssets<GpuTexture>>().get(id) else {
                return false;
            };
            draw(world, command_buffer, state, &texture.texture)
        }
        DebugSource::Swapchain => {
            copy_swapchain(world, command_buffer, state)
                && draw(
                    world,
                    command_buffer,
                    state,
                    state.swapchain_copy.as_ref().unwrap()
                )
        }
    }
}

/// Copies the swapchain texture as it is at this point of the frame.
fn copy_swapchain(
    world: &World,
    command_buffer: &mut CommandBuffer,
    state: &mut DebugState
) -> bool {
    let Some(frame) = world.resource::<SwapchainFrame>().data.as_ref() else {
        return false;
    };
    let source = &frame.texture.texture;
    if !source.usage().contains(TextureUsages::COPY_SRC) {
        return false;
    }
    let size = (source.width(), source.height());
    if state
        .swapchain_copy
        .as_ref()
        .is_none_or(|t| t.size != size || t.format != source.format())
    {
        let instance = world.resource::<RenderInstance>().0.read().unwrap();
        state.swapchain_copy = Some(wde_wgpu::texture::Texture::new(
            &instance,
            "debug-view-swapchain",
            size,
            source.format(),
            TextureUsages::TEXTURE_BINDING,
            1,
            1,
            1,
            true,
            false,
            false,
            None
        ));
    }
    command_buffer.copy_texture_to_texture(source, &state.swapchain_copy.as_ref().unwrap().texture);
    true
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct DebugViewParams {
    range: [f32; 2],
    exposure: f32,
    channels: u32,
    mode: u32,
    tonemap: u32,
    _pad: [u32; 2]
}
impl DebugViewParams {
    fn new(settings: &DebugViewSettings) -> Self {
        Self {
            range: settings.range,
            exposure: settings.exposure.exp2(),
            channels: settings.channels as u32,
            mode: settings.mode as u32,
            tonemap: settings.tonemap as u32,
            _pad: [0; 2]
        }
    }
}

#[derive(Clone, Copy)]
enum ViewKind {
    Float,
    Depth,
    Uint
}
fn view_pipeline_kind(sample_type: Option<TextureSampleType>) -> Option<ViewKind> {
    match sample_type? {
        TextureSampleType::Float { .. } => Some(ViewKind::Float),
        TextureSampleType::Depth => Some(ViewKind::Depth),
        TextureSampleType::Uint => Some(ViewKind::Uint),
        TextureSampleType::Sint => None
    }
}

fn view_layout(kind: ViewKind) -> BindGroupLayout {
    let sample_type = match kind {
        ViewKind::Float => TextureSampleType::Float { filterable: false },
        ViewKind::Depth => TextureSampleType::Depth,
        ViewKind::Uint => TextureSampleType::Uint
    };
    BindGroupLayout::new("debug-view", |builder| {
        builder.add_texture_view_sample_type(0, ShaderStages::FRAGMENT, sample_type);
    })
}

fn draw(
    world: &World,
    command_buffer: &mut CommandBuffer,
    state: &DebugState,
    source: &wde_wgpu::texture::Texture
) -> bool {
    let Some(target) = state
        .target
        .and_then(|t| world.resource::<RenderAssets<GpuTexture>>().get(t))
    else {
        return false;
    };
    let settings = &state.settings;
    let Some(kind) = view_pipeline_kind(source.sample_type(settings.stencil)) else {
        return false;
    };
    let pipeline_id = match kind {
        ViewKind::Float => pipeline_index::<DebugViewFloatPipeline>(world),
        ViewKind::Depth => pipeline_index::<DebugViewDepthPipeline>(world),
        ViewKind::Uint => pipeline_index::<DebugViewUintPipeline>(world)
    };
    let pipeline_manager = world.resource::<PipelineManager>();
    let Some(CachedPipelineStatus::OkRender(pipeline)) =
        pipeline_id.map(|id| pipeline_manager.get_pipeline(id))
    else {
        return false;
    };

    let view = source.single_view(
        settings.layer.min(source.layer_count.saturating_sub(1)),
        settings.mip.min(source.mip_level_count.saturating_sub(1)),
        settings.stencil
    );
    let bind_group = {
        let instance = world.resource::<RenderInstance>().0.read().unwrap();
        let Ok(layout) = view_layout(kind).build(&instance) else {
            return false;
        };
        let Ok(bind_group) = BindGroupBuilder::build(
            "debug-view",
            &instance,
            &layout,
            &vec![BindGroupBuilder::view(0, &view)]
        ) else {
            return false;
        };
        bind_group
    };

    let render_pass = command_buffer.create_render_pass("debug-view", |builder| {
        builder.add_color_attachment(RenderPassColorAttachment {
            texture: Some(&target.texture.view),
            load: LoadOp::Clear(WdeColor::BLACK.into()),
            store: StoreOp::Store,
            resolve_target: None
        });
        Ok(())
    });
    let mut render_pass: RenderPassInstance = match render_pass {
        Ok(render_pass) => render_pass,
        Err(e) => {
            trace!("Failed to create the debug view render pass: {}.", e);
            return false;
        }
    };
    if render_pass.set_pipeline(pipeline).is_err() {
        return false;
    }
    render_pass
        .set_bind_group(0, &bind_group)
        .set_push_constants(bytemuck::bytes_of(&DebugViewParams::new(settings)));
    render_pass.draw(0..3, 0..1).is_ok()
}

fn pipeline_index<P: RenderAsset + DebugViewPipeline>(
    world: &World
) -> Option<CachedPipelineIndex> {
    world
        .resource::<RenderAssets<P>>()
        .iter()
        .next()
        .map(|(_, p)| p.index())
}

trait DebugViewPipeline {
    fn index(&self) -> CachedPipelineIndex;
}

macro_rules! debug_view_pipeline {
    ($name:ident, $kind:expr, $label:literal, $shader:literal) => {
        #[derive(TypePath, Default, Clone, Debug)]
        pub(crate) struct $name(CachedPipelineIndex);
        impl DebugViewPipeline for $name {
            fn index(&self) -> CachedPipelineIndex {
                self.0
            }
        }
        impl RenderAsset for $name {
            type SourceAsset = RenderPipelineAsset<$name>;
            type Params = (SRes<AssetServer>, SResMut<PipelineManager>);

            fn prepare(
                _id: AssetId<Self::SourceAsset>,
                asset: Self::SourceAsset,
                (asset_server, pipeline_manager): &mut SystemParamItem<Self::Params>
            ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
                Ok($name(pipeline_manager.create_render_pipeline(
                    RenderPipelineDescriptor {
                        label: $label,
                        vert: Some(asset_server.load::<Shader>("core/render/debug/view.vert.wgsl")),
                        frag: Some(asset_server.load::<Shader>($shader)),
                        bind_group_layouts: vec![Some(view_layout($kind))],
                        push_constants: vec![PushConstantDescriptor {
                            offset: 0,
                            size: std::mem::size_of::<DebugViewParams>() as u32
                        }],
                        depth: DepthDescriptor {
                            enabled: false,
                            ..default()
                        },
                        render_targets: Some(vec![RenderGraphDebugView::TARGET_FORMAT]),
                        cull_mode: None,
                        vertex_buffer: false,
                        ..default()
                    },
                    asset
                )?))
            }
        }
    };
}
debug_view_pipeline!(
    DebugViewFloatPipeline,
    ViewKind::Float,
    "debug-view-float",
    "core/render/debug/view_float.frag.wgsl"
);
debug_view_pipeline!(
    DebugViewDepthPipeline,
    ViewKind::Depth,
    "debug-view-depth",
    "core/render/debug/view_depth.frag.wgsl"
);
debug_view_pipeline!(
    DebugViewUintPipeline,
    ViewKind::Uint,
    "debug-view-uint",
    "core/render/debug/view_uint.frag.wgsl"
);
