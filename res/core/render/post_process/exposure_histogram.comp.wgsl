// Bins the log2 luminance of the render texture into the exposure histogram.
#include "core/render/post_process/post_process_params.wgsl"

@group(0) @binding(0) var in_hdr: texture_2d<f32>;
@group(0) @binding(1) var<uniform> in_params: PostProcessParams;
@group(0) @binding(2) var<storage, read_write> out_histogram: array<atomic<u32>, EXPOSURE_HISTOGRAM_BINS>;

var<workgroup> local_bins: array<atomic<u32>, EXPOSURE_HISTOGRAM_BINS>;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>, @builtin(local_invocation_index) index: u32) {
    if index < EXPOSURE_HISTOGRAM_BINS {
        atomicStore(&local_bins[index], 0u);
    }
    workgroupBarrier();

    if all(id.xy < textureDimensions(in_hdr)) {
        let lum = luminance(textureLoad(in_hdr, id.xy, 0).rgb);
        let t = (log2(max(lum, 1e-6)) - in_params.exposure_metering.x) / in_params.exposure_metering.y;
        let bin = u32(clamp(t, 0.0, 1.0) * f32(EXPOSURE_HISTOGRAM_BINS - 1u) + 0.5);
        atomicAdd(&local_bins[min(bin, EXPOSURE_HISTOGRAM_BINS - 1u)], 1u);
    }
    workgroupBarrier();

    if index < EXPOSURE_HISTOGRAM_BINS {
        atomicAdd(&out_histogram[index], atomicLoad(&local_bins[index]));
    }
}
