//! Timings of the frames and of the passes of the render graph, shown by its debug view (see
//! [`RenderGraphDebugView`](super::RenderGraphDebugView)).
use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant}
};

use wde_wgpu::{
    command_buffer::CommandBuffer,
    instance::{PresentMode, RenderInstanceData},
    timestamps::GpuTimestamps
};

use super::{DebugPassReport, RenderPassId};

/// Most passes timed on the GPU in a frame.
const MAX_GPU_PASSES: u32 = 128;

/// Average, minimum and maximum of a duration over the last frames, in milliseconds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DebugTiming {
    pub avg: f32,
    pub min: f32,
    pub max: f32
}

/// Timings of the last frames, over the [`RenderGraphDebugView::timing_window`](super::RenderGraphDebugView).
#[derive(Clone, Debug, Default)]
pub struct DebugFrameTimings {
    /// Interval between two presented frames.
    pub frame: Option<DebugTiming>,
    /// Render schedule of the render thread, swapchain waits included.
    pub render: Option<DebugTiming>,
    /// Render schedule, swapchain waits excluded. Approximate, as other systems may run during the waits.
    pub render_busy: Option<DebugTiming>,
    /// Time blocked acquiring and presenting the swapchain texture: waiting for the present mode and the GPU.
    pub swapchain_wait: Option<DebugTiming>,
    /// Recording of the render graph commands.
    pub graph_cpu: Option<DebugTiming>,
    /// Render graph on the GPU, from the start of its first pass to the end of its last.
    pub graph_gpu: Option<DebugTiming>,
    pub present_mode: Option<PresentMode>,
    /// Whether the device writes timestamps. Without them, the GPU timings stay empty.
    pub gpu_supported: bool,
    /// Last frames, oldest first: the CPU time of each pass in their execution order, in ms.
    pub cpu_history: VecDeque<Vec<(RenderPassId, f32)>>,
    /// Same as the `cpu_history` for the GPU times, a few frames late.
    pub gpu_history: VecDeque<Vec<(RenderPassId, f32)>>,
    /// Intervals between the last presented frames, oldest first, in ms.
    pub frame_history: VecDeque<f32>
}

fn ms(duration: Duration) -> f32 {
    duration.as_secs_f32() * 1000.0
}

fn push_bounded<T>(values: &mut VecDeque<T>, value: T, len: usize) {
    while values.len() >= len.max(1) {
        values.pop_front();
    }
    values.push_back(value);
}

#[derive(Default)]
struct TimingWindow(VecDeque<f32>);
impl TimingWindow {
    fn push(&mut self, duration: Duration, len: usize) {
        push_bounded(&mut self.0, ms(duration), len);
    }

    fn summary(&self) -> Option<DebugTiming> {
        if self.0.is_empty() {
            return None;
        }
        let (sum, min, max) = self
            .0
            .iter()
            .fold((0.0, f32::MAX, f32::MIN), |(s, lo, hi), v| {
                (s + v, lo.min(*v), hi.max(*v))
            });
        Some(DebugTiming {
            avg: sum / self.0.len() as f32,
            min,
            max
        })
    }
}

#[derive(Default)]
struct PassTimings {
    cpu: TimingWindow,
    gpu: TimingWindow
}

/// Timings measured in the render world, while the debug view is enabled.
#[derive(Default)]
pub(super) struct Timings {
    /// Number of frames the timings are computed over.
    pub(super) window: usize,
    /// `None` until created by the first frame, then `Some(None)` if the device has no timestamps.
    gpu: Option<Option<GpuTimestamps<Vec<RenderPassId>>>>,
    /// Passes timed on the GPU this frame, by two timestamps each.
    gpu_passes: Vec<RenderPassId>,
    pass_start: Option<(RenderPassId, Instant, bool)>,
    frame_start: Option<Instant>,
    graph_start: Option<Instant>,
    swapchain_wait: Duration,
    last_present: Option<Instant>,

    frame: TimingWindow,
    render: TimingWindow,
    render_busy: TimingWindow,
    swapchain_waits: TimingWindow,
    graph_cpu: TimingWindow,
    graph_gpu: TimingWindow,
    passes: HashMap<RenderPassId, PassTimings>,
    /// CPU times of the passes of this frame.
    frame_cpu: Vec<(RenderPassId, f32)>,
    cpu_history: VecDeque<Vec<(RenderPassId, f32)>>,
    gpu_history: VecDeque<Vec<(RenderPassId, f32)>>
}

impl Timings {
    pub(super) fn begin_frame(&mut self, instance: &RenderInstanceData) {
        self.frame_start = Some(Instant::now());
        self.swapchain_wait = Duration::ZERO;

        let window = self.window;
        let Some(gpu) = self
            .gpu
            .get_or_insert_with(|| {
                GpuTimestamps::new(instance, "render-graph-timestamps", MAX_GPU_PASSES * 2)
            })
            .as_mut()
        else {
            return;
        };
        for (passes, timestamps) in gpu.collect(instance) {
            let duration = |start: usize| {
                Duration::from_nanos(timestamps[start + 1].saturating_sub(timestamps[start]))
            };
            let mut frame = Vec::with_capacity(passes.len());
            for (i, id) in passes.iter().take(timestamps.len() / 2).enumerate() {
                let duration = duration(2 * i);
                self.passes
                    .entry(*id)
                    .or_default()
                    .gpu
                    .push(duration, window);
                frame.push((*id, ms(duration)));
            }
            push_bounded(&mut self.gpu_history, frame, window);
            if let (Some(first), Some(last)) = (timestamps.first(), timestamps.last()) {
                self.graph_gpu
                    .push(Duration::from_nanos(last.saturating_sub(*first)), window);
            }
        }
    }

    pub(super) fn add_swapchain_wait(&mut self, wait: Duration) {
        self.swapchain_wait += wait;
    }

    pub(super) fn presented(&mut self, wait: Duration) {
        self.swapchain_wait += wait;
        let now = Instant::now();
        if let Some(last) = self.last_present.replace(now) {
            self.frame.push(now - last, self.window);
        }
    }

    pub(super) fn end_frame(&mut self) {
        let Some(start) = self.frame_start.take() else {
            return;
        };
        let render = start.elapsed();
        self.render.push(render, self.window);
        self.render_busy
            .push(render.saturating_sub(self.swapchain_wait), self.window);
        self.swapchain_waits.push(self.swapchain_wait, self.window);
    }

    pub(super) fn begin_graph(&mut self) {
        self.graph_start = Some(Instant::now());
        self.gpu_passes.clear();
        self.frame_cpu.clear();
    }

    pub(super) fn begin_pass(&mut self, command_buffer: &mut CommandBuffer, id: RenderPassId) {
        let gpu = self
            .gpu
            .as_mut()
            .and_then(Option::as_mut)
            .and_then(|gpu| gpu.write(command_buffer))
            .is_some();
        self.pass_start = Some((id, Instant::now(), gpu));
    }

    pub(super) fn end_pass(&mut self, command_buffer: &mut CommandBuffer) {
        let Some((id, start, gpu)) = self.pass_start.take() else {
            return;
        };
        let window = self.window;
        let elapsed = start.elapsed();
        self.passes.entry(id).or_default().cpu.push(elapsed, window);
        self.frame_cpu.push((id, ms(elapsed)));
        if gpu && let Some(Some(timestamps)) = self.gpu.as_mut() {
            timestamps.write(command_buffer);
            self.gpu_passes.push(id);
        }
    }

    pub(super) fn end_graph(
        &mut self,
        instance: &RenderInstanceData,
        command_buffer: &mut CommandBuffer
    ) {
        if let Some(start) = self.graph_start.take() {
            self.graph_cpu.push(start.elapsed(), self.window);
        }
        let frame = std::mem::take(&mut self.frame_cpu);
        push_bounded(&mut self.cpu_history, frame, self.window);
        if let Some(Some(gpu)) = self.gpu.as_mut() {
            gpu.resolve(
                instance,
                command_buffer,
                std::mem::take(&mut self.gpu_passes)
            );
        }
    }

    pub(super) fn submitted(&mut self) {
        if let Some(Some(gpu)) = self.gpu.as_mut() {
            gpu.map();
        }
    }

    pub(super) fn fill_reports(&self, reports: &mut [DebugPassReport]) {
        for report in reports {
            if let Some(timings) = self.passes.get(&report.id) {
                report.cpu_time = timings.cpu.summary();
                report.gpu_time = timings.gpu.summary();
            }
        }
    }

    pub(super) fn summary(&self, present_mode: Option<PresentMode>) -> DebugFrameTimings {
        DebugFrameTimings {
            frame: self.frame.summary(),
            render: self.render.summary(),
            render_busy: self.render_busy.summary(),
            swapchain_wait: self.swapchain_waits.summary(),
            graph_cpu: self.graph_cpu.summary(),
            graph_gpu: self.graph_gpu.summary(),
            present_mode,
            gpu_supported: matches!(self.gpu, Some(Some(_))),
            cpu_history: self.cpu_history.clone(),
            gpu_history: self.gpu_history.clone(),
            frame_history: self.frame.0.clone()
        }
    }
}
