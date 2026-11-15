//! GPU timestamps written between the commands of a command buffer, read back without blocking.
use std::{
    collections::VecDeque,
    sync::mpsc::{self, TryRecvError}
};

use crate::{command_buffer::CommandBuffer, instance::RenderInstanceData, memory::GpuAllocation};

/// Resolved frames waiting to be read back, beyond which new frames are dropped.
const MAX_PENDING: usize = 4;

/// Device features needed by [`GpuTimestamps`].
pub fn timestamp_features() -> wgpu::Features {
    wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
}

/// Timestamps written into a command buffer with [`GpuTimestamps::write`], resolved at the end of each frame with
/// [`GpuTimestamps::resolve`] and read back a few frames later with [`GpuTimestamps::collect`]. Each frame carries
/// a `T` describing what its timestamps measure.
pub struct GpuTimestamps<T> {
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    _resolve_memory: GpuAllocation,
    capacity: u32,
    /// Timestamps written in the current frame.
    count: u32,
    /// Nanoseconds per tick.
    period: f64,
    free: Vec<(wgpu::Buffer, GpuAllocation)>,
    pending: VecDeque<PendingFrame<T>>
}

struct PendingFrame<T> {
    buffer: wgpu::Buffer,
    memory: GpuAllocation,
    count: u32,
    data: T,
    /// Set once the buffer mapping is requested.
    mapped: Option<mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>>
}

impl<T> GpuTimestamps<T> {
    /// `None` if the device lacks the [`timestamp_features`].
    pub fn new(instance: &RenderInstanceData, label: &str, capacity: u32) -> Option<Self> {
        if !instance.device.features().contains(timestamp_features()) || capacity == 0 {
            return None;
        }
        let query_set = instance.device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some(label),
            ty: wgpu::QueryType::Timestamp,
            count: capacity
        });
        let resolve_buffer = instance.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{label}-resolve")),
            size: Self::size(capacity),
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false
        });
        Some(Self {
            query_set,
            _resolve_memory: GpuAllocation::buffer(&format!("{label}-resolve"), &resolve_buffer),
            resolve_buffer,
            capacity,
            count: 0,
            period: instance.queue.get_timestamp_period() as f64,
            free: vec![],
            pending: VecDeque::new()
        })
    }

    fn size(count: u32) -> u64 {
        count as u64 * wgpu::QUERY_SIZE as u64
    }

    /// Writes a timestamp once the previous commands are done. Returns its index in the frame, or `None` once the
    /// capacity is reached.
    pub fn write(&mut self, command_buffer: &mut CommandBuffer) -> Option<u32> {
        if self.count >= self.capacity {
            return None;
        }
        command_buffer
            .encoder()
            .write_timestamp(&self.query_set, self.count);
        self.count += 1;
        Some(self.count - 1)
    }

    /// Copies the timestamps of the frame to be read back, with its `data`. Call [`GpuTimestamps::map`] once the
    /// command buffer is submitted.
    pub fn resolve(
        &mut self,
        instance: &RenderInstanceData,
        command_buffer: &mut CommandBuffer,
        data: T
    ) {
        let count = std::mem::take(&mut self.count);
        if count == 0 || self.pending.len() >= MAX_PENDING {
            return;
        }
        let (buffer, memory) = self.free.pop().unwrap_or_else(|| {
            let buffer = instance.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("timestamps-readback"),
                size: Self::size(self.capacity),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false
            });
            let memory = GpuAllocation::buffer("timestamps-readback", &buffer);
            (buffer, memory)
        });
        let encoder = command_buffer.encoder();
        encoder.resolve_query_set(&self.query_set, 0..count, &self.resolve_buffer, 0);
        encoder.copy_buffer_to_buffer(&self.resolve_buffer, 0, &buffer, 0, Self::size(count));
        self.pending.push_back(PendingFrame {
            buffer,
            memory,
            count,
            data,
            mapped: None
        });
    }

    /// Starts reading back the resolved frames. Call it after submitting their command buffer.
    pub fn map(&mut self) {
        for frame in self.pending.iter_mut().filter(|f| f.mapped.is_none()) {
            let (sender, receiver) = mpsc::channel();
            frame.buffer.slice(..Self::size(frame.count)).map_async(
                wgpu::MapMode::Read,
                move |result| {
                    let _ = sender.send(result);
                }
            );
            frame.mapped = Some(receiver);
        }
    }

    /// Frames read back since the last call, oldest first, with their timestamps in nanoseconds. Polls the device
    /// without blocking.
    pub fn collect(&mut self, instance: &RenderInstanceData) -> Vec<(T, Vec<u64>)> {
        instance.poll_non_blocking();
        let mut frames = vec![];
        while let Some(result) = self
            .pending
            .front()
            .and_then(|f| f.mapped.as_ref())
            .map(|r| r.try_recv())
        {
            if result == Err(TryRecvError::Empty) {
                break;
            }
            let frame = self.pending.pop_front().unwrap();
            if result != Ok(Ok(())) {
                continue;
            }
            let slice = frame.buffer.slice(..Self::size(frame.count));
            let timestamps = slice
                .get_mapped_range()
                .chunks_exact(wgpu::QUERY_SIZE as usize)
                .map(|t| (u64::from_ne_bytes(t.try_into().unwrap()) as f64 * self.period) as u64)
                .collect();
            frame.buffer.unmap();
            frames.push((frame.data, timestamps));
            self.free.push((frame.buffer, frame.memory));
        }
        frames
    }
}
