//! Status of the pipelines and their shaders, and the parts of passes skipped each frame (see
//! [`PipelineStatusView`]).
use std::sync::Mutex;

use bevy::prelude::*;

use super::{CachedPipelineIndex, PipelineManager, RenderPassId};
use crate::{
    core::RenderApp,
    sync::{ExtractSchedule, MainWorld}
};

pub(crate) struct PipelineStatusPlugin;
impl Plugin for PipelineStatusPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PipelineStatusView>();
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<PipelineStatus>()
            .add_systems(ExtractSchedule, extract);
    }
}

// ── MAIN WORLD ──────────────────────────────────────────────────

/// Status of the pipelines and their shaders, and the parts of passes skipped in the last frame. The
/// `Debug/Pipeline Status` window of the editor shows it.
#[derive(Resource, Default, Clone)]
pub struct PipelineStatusView {
    /// Collects the reports below. Disable it when unused.
    pub enabled: bool,
    /// Sorted by status, failed ones first.
    pub pipelines: Vec<DebugPipelineReport>,
    /// Parts of passes skipped in the last rendered frame, in their execution order.
    pub skips: Vec<DebugSkip>
}

#[derive(Clone, Debug)]
pub struct DebugPipelineReport {
    /// `None` while waiting for its bind group layouts.
    pub id: Option<CachedPipelineIndex>,
    pub label: &'static str,
    pub compute: bool,
    pub status: DebugPipelineStatus,
    pub shaders: Vec<DebugShaderReport>
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DebugPipelineStatus {
    /// Failed to build, with its error. It is rebuilt when one of its shaders changes.
    Failed(String),
    /// Not queued yet, as a bind group layout is missing.
    WaitingLayouts,
    /// Waiting for its shaders to load.
    Pending,
    Ready
}
impl DebugPipelineStatus {
    pub(crate) fn rank(&self) -> u8 {
        match self {
            Self::Failed(_) => 0,
            Self::WaitingLayouts => 1,
            Self::Pending => 2,
            Self::Ready => 3
        }
    }
}

#[derive(Clone, Debug)]
pub struct DebugShaderReport {
    pub path: String,
    pub status: DebugShaderStatus
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DebugShaderStatus {
    Loading,
    Loaded,
    /// Failed to load (missing file or include), with its error.
    Failed(String)
}

/// A part of a pass skipped in a frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugSkip {
    pub pass: RenderPassId,
    pub pass_label: &'static str,
    /// `None` for the whole pass, or a step of a custom pass.
    pub sub_pass: Option<&'static str>,
    pub reason: DebugSkipReason,
    /// Times it was skipped in the frame.
    pub count: u32
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DebugSkipReason {
    /// A pipeline was requested while pending or failed.
    Pipeline(CachedPipelineIndex),
    /// A sub-pass has no pipeline yet, as its pipeline asset is not prepared.
    NoPipeline,
    NoBindGroup(u32),
    NoMesh,
    /// The render pass could not be created.
    RenderPass(String),
    /// The custom render of the pass did not complete.
    CustomRender
}
impl std::fmt::Display for DebugSkipReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pipeline(id) => write!(f, "pipeline {id} not ready"),
            Self::NoPipeline => f.write_str("no pipeline"),
            Self::NoBindGroup(index) => write!(f, "no bind group {index}"),
            Self::NoMesh => f.write_str("mesh not loaded"),
            Self::RenderPass(error) => write!(f, "render pass: {error}"),
            Self::CustomRender => f.write_str("custom render incomplete")
        }
    }
}

// ── RENDER WORLD ────────────────────────────────────────────────

/// Records the parts of passes skipped in the frame being rendered (see [`PipelineStatusView`]).
#[derive(Resource, Default)]
pub(crate) struct PipelineStatus(Mutex<SkipState>);

#[derive(Default)]
struct SkipState {
    enabled: bool,
    pass: Option<(RenderPassId, &'static str)>,
    skips: Vec<DebugSkip>,
    /// Skips of the last rendered frame, sent to the main world.
    finished: Option<Vec<DebugSkip>>
}
impl SkipState {
    fn add(&mut self, sub_pass: Option<&'static str>, reason: DebugSkipReason) {
        let Some((pass, pass_label)) = self.pass else {
            return;
        };
        match self
            .skips
            .iter_mut()
            .find(|s| s.pass == pass && s.sub_pass == sub_pass && s.reason == reason)
        {
            Some(skip) => skip.count += 1,
            None => self.skips.push(DebugSkip {
                pass,
                pass_label,
                sub_pass,
                reason,
                count: 1
            })
        }
    }
}

fn extract(
    mut main_world: ResMut<MainWorld>,
    status: Res<PipelineStatus>,
    manager: Res<PipelineManager>,
    asset_server: Res<AssetServer>
) {
    let mut view = main_world.resource_mut::<PipelineStatusView>();
    let mut state = status.0.lock().unwrap();
    state.enabled = view.enabled;
    if !view.enabled {
        return;
    }
    view.pipelines = manager.report(&asset_server);
    if let Some(skips) = state.finished.take() {
        view.skips = skips;
    }
}

impl PipelineStatus {
    pub(crate) fn begin_pass(world: &World, id: RenderPassId, label: &'static str) {
        // Pipelines requested outside of the passes are not reported
        world.resource::<PipelineManager>().take_misses();
        world.resource::<PipelineStatus>().0.lock().unwrap().pass = Some((id, label));
    }

    pub(crate) fn skip(world: &World, sub_pass: Option<&'static str>, reason: DebugSkipReason) {
        let mut state = world.resource::<PipelineStatus>().0.lock().unwrap();
        if state.enabled {
            state.add(sub_pass, reason);
        }
    }

    /// Reports the pipelines requested by the pass while not ready, not reported by its sub-passes.
    pub(crate) fn end_pass(world: &World) {
        let misses = world.resource::<PipelineManager>().take_misses();
        let mut state = world.resource::<PipelineStatus>().0.lock().unwrap();
        if state.enabled {
            let pass = state.pass.map(|(id, _)| id);
            for id in misses {
                let reason = DebugSkipReason::Pipeline(id);
                if !state
                    .skips
                    .iter()
                    .any(|s| Some(s.pass) == pass && s.reason == reason)
                {
                    state.add(None, reason);
                }
            }
        }
        state.pass = None;
    }

    pub(crate) fn end_frame(world: &World) {
        let mut state = world.resource::<PipelineStatus>().0.lock().unwrap();
        if state.enabled {
            state.finished = Some(std::mem::take(&mut state.skips));
        } else {
            state.skips.clear();
        }
    }
}
