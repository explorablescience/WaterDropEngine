// Averages the exposure histogram between the metering percentiles, then moves the adapted exposure
// towards the one bringing this average to middle grey. Clears the histogram for the next frame.
#include "core/render/post_process/post_process_params.wgsl"

@group(0) @binding(1) var<uniform> in_params: PostProcessParams;
@group(0) @binding(2) var<storage, read_write> io_histogram: array<atomic<u32>, EXPOSURE_HISTOGRAM_BINS>;
@group(0) @binding(3) var<storage, read_write> io_exposure: ExposureState;

var<workgroup> bins: array<u32, EXPOSURE_HISTOGRAM_BINS>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(local_invocation_index) index: u32) {
    bins[index] = atomicExchange(&io_histogram[index], 0u);
    workgroupBarrier();
    if index != 0u {
        return;
    }

    var total = 0u;
    for (var i = 0u; i < EXPOSURE_HISTOGRAM_BINS; i++) {
        total += bins[i];
    }
    if total == 0u {
        return;
    }

    // Pixel ranks kept by the metering, at least one pixel wide
    let metering = in_params.exposure_metering;
    let n = f32(total);
    let low = min(n * metering.z, n - 1.0);
    let high = max(n * metering.w, low + 1.0);

    var below = 0.0;
    var sum = 0.0;
    var weight = 0.0;
    for (var i = 0u; i < EXPOSURE_HISTOGRAM_BINS; i++) {
        let count = f32(bins[i]);
        let kept = max(min(below + count, high) - max(below, low), 0.0);
        below += count;
        sum += kept * (metering.x + f32(i) / f32(EXPOSURE_HISTOGRAM_BINS - 1u) * metering.y);
        weight += kept;
    }

    let target_ev = clamp(log2(0.18) - sum / weight, in_params.exposure_limits.x, in_params.exposure_limits.y);
    if io_exposure.initialized == 0u {
        io_exposure.ev = target_ev;
        io_exposure.initialized = 1u;
        return;
    }
    let speed = select(in_params.exposure.w, in_params.exposure.z, target_ev > io_exposure.ev);
    io_exposure.ev += (target_ev - io_exposure.ev) * (1.0 - exp(-speed * in_params.exposure.y));
}
