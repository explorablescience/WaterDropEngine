//! On-demand compute dispatch for callers outside the render world's own per-frame systems (e.g.
//! background tasks). Unlike [`crate::core::PipelineManager`], which builds pipelines
//! asynchronously across frames and is only usable from render-app ECS systems, this builds
//! synchronously and caches by label, so it's safe to call from any thread and cheap to call
//! repeatedly.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use wde_wgpu::bind_group::{BindGroupBuilder, BindGroupLayout, WgpuBindGroupLayout};
use wde_wgpu::buffer::{Buffer, BufferBindingType, BufferUsage};
use wde_wgpu::command_buffer::CommandBuffer;
use wde_wgpu::compute_pipeline::ComputePipeline;
use wde_wgpu::render_pipeline::ShaderStages;

use crate::core::RenderInstance;

struct CachedCompute {
    pipeline: ComputePipeline,
    layout: WgpuBindGroupLayout
}

/// Caches a compiled pipeline + bind group layout per label, so repeat dispatches skip WGSL
/// parsing/validation. Blocks the calling thread until the result is read back.
pub struct ComputeDispatcher {
    instance: RenderInstance,
    cache: Mutex<HashMap<&'static str, Arc<CachedCompute>>>
}
impl ComputeDispatcher {
    pub fn new(instance: &RenderInstance) -> Self {
        Self {
            instance: RenderInstance(instance.0.clone()),
            cache: Mutex::new(HashMap::new())
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

        let cached = {
            let mut cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(label) {
                cached.clone()
            } else {
                let layout = BindGroupLayout::new(label, |b| {
                    b.add_buffer(0, ShaderStages::COMPUTE, BufferBindingType::Uniform);
                    b.add_buffer(
                        1,
                        ShaderStages::COMPUTE,
                        BufferBindingType::Storage { read_only: false }
                    );
                    for i in 0..inputs.len() {
                        b.add_buffer(
                            2 + i as u32,
                            ShaderStages::COMPUTE,
                            BufferBindingType::Storage { read_only: true }
                        );
                    }
                });
                let wgpu_layout = layout.build(&data).map_err(|e| format!("{label}: {e:?}"))?;
                let mut pipeline = ComputePipeline::new(label);
                pipeline
                    .set_shader(shader)
                    .set_bind_groups(vec![wgpu_layout.clone()]);
                pipeline.init(&data).map_err(|e| format!("{label}: {e:?}"))?;
                let cached = Arc::new(CachedCompute {
                    pipeline,
                    layout: wgpu_layout
                });
                cache.insert(label, cached.clone());
                cached
            }
        };

        let params_buf = Buffer::new(
            &data,
            &format!("{label}-params"),
            std::mem::size_of::<P>(),
            BufferUsage::UNIFORM | BufferUsage::COPY_DST,
            Some(bytemuck::bytes_of(params))
        );
        let output_size = output_len * std::mem::size_of::<f32>();
        let output_buf = Buffer::new(
            &data,
            &format!("{label}-output"),
            output_size,
            BufferUsage::STORAGE | BufferUsage::COPY_SRC,
            None
        );
        let input_bufs: Vec<Buffer> = inputs
            .iter()
            .enumerate()
            .map(|(i, slice)| {
                Buffer::new(
                    &data,
                    &format!("{label}-input{i}"),
                    std::mem::size_of_val(*slice),
                    BufferUsage::STORAGE | BufferUsage::COPY_DST,
                    Some(bytemuck::cast_slice(slice))
                )
            })
            .collect();

        let mut entries = vec![
            BindGroupBuilder::buffer(0, &params_buf),
            BindGroupBuilder::buffer(1, &output_buf),
        ];
        for (i, buf) in input_bufs.iter().enumerate() {
            entries.push(BindGroupBuilder::buffer(2 + i as u32, buf));
        }
        let bind_group = BindGroupBuilder::build(label, &data, &cached.layout, &entries)
            .map_err(|e| format!("{label}: {e:?}"))?;

        let mut cmd = CommandBuffer::new(&data, label);
        {
            let mut pass = cmd.create_compute_pass(label);
            pass.set_pipeline(&cached.pipeline)
                .map_err(|e| format!("{label}: {e:?}"))?
                .set_bind_group(0, &bind_group);
            pass.dispatch(workgroups.0, workgroups.1, workgroups.2)
                .map_err(|e| format!("{label}: {e:?}"))?;
        }
        cmd.submit(&data);

        let staging = Buffer::new(
            &data,
            &format!("{label}-staging"),
            output_size,
            BufferUsage::MAP_READ | BufferUsage::COPY_DST,
            None
        );
        staging.copy_from_buffer(&data, &output_buf);

        let mut result = vec![0.0f32; output_len];
        staging.map_read(&data, |view| {
            result.copy_from_slice(bytemuck::cast_slice(&view));
        });
        Ok(result)
    }
}
