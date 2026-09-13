//! On-demand compute dispatch for callers outside the render world's own per-frame systems (e.g.
//! background tasks). Unlike [`crate::core::PipelineManager`], which builds pipelines
//! asynchronously across frames and is only usable from render-app ECS systems, this builds
//! synchronously and caches by label, so it's safe to call from any thread and cheap to call
//! repeatedly.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use wde_wgpu::bind_group::{BindGroup, BindGroupBuilder, BindGroupLayout, WgpuBindGroupLayout};
use wde_wgpu::buffer::{Buffer, BufferBindingType, BufferUsage};
use wde_wgpu::command_buffer::CommandBuffer;
use wde_wgpu::compute_pipeline::ComputePipeline;
use wde_wgpu::instance::RenderInstanceData;
use wde_wgpu::render_pipeline::ShaderStages;

use crate::core::RenderInstance;

struct CachedPipeline {
    pipeline: ComputePipeline,
    layout: WgpuBindGroupLayout
}

/// One dispatch's GPU-side working set, checked out of the pool for the call's duration.
struct BufferSet {
    params: Buffer,
    output: Buffer,
    inputs: Vec<Buffer>,
    staging: Buffer,
    bind_group: BindGroup
}

/// Identifies interchangeable buffer sets: same label, params size, output/input lengths.
#[derive(PartialEq, Eq, Hash, Clone)]
struct BufferShape {
    label: &'static str,
    params_size: usize,
    output_len: usize,
    input_lens: Vec<usize>
}

/// Caches a compiled pipeline per label and pools GPU buffers (+ bind group) by shape, so a
/// repeat dispatch only re-uploads buffer contents. Blocks until the result is read back.
pub struct ComputeDispatcher {
    instance: RenderInstance,
    pipelines: Mutex<HashMap<&'static str, Arc<CachedPipeline>>>,
    buffer_pool: Mutex<HashMap<BufferShape, Vec<BufferSet>>>
}
impl ComputeDispatcher {
    pub fn new(instance: &RenderInstance) -> Self {
        Self {
            instance: RenderInstance(instance.0.clone()),
            pipelines: Mutex::new(HashMap::new()),
            buffer_pool: Mutex::new(HashMap::new())
        }
    }

    /// Dispatches `shader`'s `main` over `output_len` `f32`s (binding 1), with `params` as a
    /// uniform buffer (binding 0) and `inputs` as read-only `f32` storage buffers at bindings
    /// `2, 3, ...`. `inputs.len()` must stay the same across calls sharing `label`, since the
    /// bind group layout is built once and cached under it.
    pub fn dispatch_f32<P: bytemuck::NoUninit>(
        &self,
        label: &'static str,
        shader: &str,
        params: &P,
        inputs: &[&[f32]],
        output_len: usize,
        workgroups: (u32, u32, u32)
    ) -> Result<Vec<f32>, String> {
        let data = self
            .instance
            .0
            .read()
            .map_err(|_| "render instance lock poisoned".to_string())?;

        let pipeline = self.pipeline_for(label, shader, inputs.len(), &data)?;
        let shape = BufferShape {
            label,
            params_size: std::mem::size_of::<P>(),
            output_len,
            input_lens: inputs.iter().map(|slice| slice.len()).collect()
        };
        let buffers = self.checkout_buffers(shape.clone(), &pipeline, &data)?;

        buffers.params.write(&data, bytemuck::bytes_of(params), 0);
        for (buf, input) in buffers.inputs.iter().zip(inputs) {
            buf.write(&data, bytemuck::cast_slice(input), 0);
        }

        let mut cmd = CommandBuffer::new(&data, label);
        {
            let mut pass = cmd.create_compute_pass(label);
            pass.set_pipeline(&pipeline.pipeline)
                .map_err(|e| format!("{label}: {e:?}"))?
                .set_bind_group(0, &buffers.bind_group);
            pass.dispatch(workgroups.0, workgroups.1, workgroups.2)
                .map_err(|e| format!("{label}: {e:?}"))?;
        }
        cmd.submit(&data);

        buffers.staging.copy_from_buffer(&data, &buffers.output);
        let mut result = vec![0.0f32; output_len];
        buffers.staging.map_read(&data, |view| {
            result.copy_from_slice(bytemuck::cast_slice(&view));
        });

        self.buffer_pool
            .lock()
            .unwrap()
            .entry(shape)
            .or_default()
            .push(buffers);
        Ok(result)
    }

    fn pipeline_for(
        &self,
        label: &'static str,
        shader: &str,
        input_count: usize,
        data: &RenderInstanceData
    ) -> Result<Arc<CachedPipeline>, String> {
        let mut cache = self.pipelines.lock().unwrap();
        if let Some(cached) = cache.get(label) {
            return Ok(cached.clone());
        }

        let layout = BindGroupLayout::new(label, |b| {
            b.add_buffer(0, ShaderStages::COMPUTE, BufferBindingType::Uniform);
            b.add_buffer(
                1,
                ShaderStages::COMPUTE,
                BufferBindingType::Storage { read_only: false }
            );
            for i in 0..input_count {
                b.add_buffer(
                    2 + i as u32,
                    ShaderStages::COMPUTE,
                    BufferBindingType::Storage { read_only: true }
                );
            }
        });
        let wgpu_layout = layout.build(data).map_err(|e| format!("{label}: {e:?}"))?;
        let mut pipeline = ComputePipeline::new(label);
        pipeline
            .set_shader(shader)
            .set_bind_groups(vec![wgpu_layout.clone()]);
        pipeline.init(data).map_err(|e| format!("{label}: {e:?}"))?;

        let cached = Arc::new(CachedPipeline {
            pipeline,
            layout: wgpu_layout
        });
        cache.insert(label, cached.clone());
        Ok(cached)
    }

    /// Pops an idle buffer set matching `shape`, or builds a fresh one.
    fn checkout_buffers(
        &self,
        shape: BufferShape,
        pipeline: &CachedPipeline,
        data: &RenderInstanceData
    ) -> Result<BufferSet, String> {
        if let Some(set) = self
            .buffer_pool
            .lock()
            .unwrap()
            .get_mut(&shape)
            .and_then(Vec::pop)
        {
            return Ok(set);
        }

        let label = shape.label;
        let params = Buffer::new(
            data,
            &format!("{label}-params"),
            shape.params_size,
            BufferUsage::UNIFORM | BufferUsage::COPY_DST,
            None
        );
        let output_size = shape.output_len * std::mem::size_of::<f32>();
        let output = Buffer::new(
            data,
            &format!("{label}-output"),
            output_size,
            BufferUsage::STORAGE | BufferUsage::COPY_SRC,
            None
        );
        let inputs: Vec<Buffer> = shape
            .input_lens
            .iter()
            .enumerate()
            .map(|(i, &len)| {
                Buffer::new(
                    data,
                    &format!("{label}-input{i}"),
                    len * std::mem::size_of::<f32>(),
                    BufferUsage::STORAGE | BufferUsage::COPY_DST,
                    None
                )
            })
            .collect();
        let staging = Buffer::new(
            data,
            &format!("{label}-staging"),
            output_size,
            BufferUsage::MAP_READ | BufferUsage::COPY_DST,
            None
        );

        let mut entries = vec![
            BindGroupBuilder::buffer(0, &params),
            BindGroupBuilder::buffer(1, &output),
        ];
        for (i, buf) in inputs.iter().enumerate() {
            entries.push(BindGroupBuilder::buffer(2 + i as u32, buf));
        }
        let bind_group = BindGroupBuilder::build(label, data, &pipeline.layout, &entries)
            .map_err(|e| format!("{label}: {e:?}"))?;

        Ok(BufferSet {
            params,
            output,
            inputs,
            staging,
            bind_group
        })
    }
}
