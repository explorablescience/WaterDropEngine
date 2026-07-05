// Push constants: per-tile metadata sent from the CPU each dispatch.
// WGSL struct alignment: max member align = 8 (vec2<f32>), size rounds to 32 bytes.
struct TileInfo {
    tile_idx:          vec2<f32>,
    tile_size:         vec2<f32>,
    tile_subdivisions: f32,
    tile_layer:        u32,
    commands_count:    u32,
    // implicit 4-byte padding to reach 32 bytes
}
var<push_constant> in_tile_info: TileInfo;

// One entry per paint command uploaded from the CPU this frame.
// Layout must match CommandDescription in commands_buffer.rs (48 bytes).
struct Command {
    world_position: vec2<f32>,  // x, z in world space
    radius:         f32,
    strength:       f32,
    color:          vec4<f32>,
    brush_type:     f32,        // 0=Paint 1=Erase 2=Raise 3=Lower 4=Smooth 5=Flatten
    target_height:  f32,        // normalized [0,1] target for Flatten mode
    _padding:       vec2<f32>,
}
@group(0) @binding(0) var<storage, read> in_commands: array<Command>;

@group(1) @binding(0) var in_heightmap: texture_storage_2d_array<r8unorm,    read_write>;
@group(1) @binding(1) var in_splatmap:  texture_storage_2d_array<rgba8unorm, read_write>;

// Each thread owns one pixel and iterates over all commands.
// Dispatch: (tile_subdiv * tile_subdiv / 64) workgroups per tile — constant regardless
// of command count, avoiding the O(N_commands * N_pixels) over-dispatch of the old scheme.
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let tile_subdiv = u32(in_tile_info.tile_subdivisions);
    let layer       = i32(in_tile_info.tile_layer);
    let pixel_index = global_id.x;
    let idx         = vec2<u32>(pixel_index % tile_subdiv, pixel_index / tile_subdiv);

    if idx.x >= tile_subdiv || idx.y >= tile_subdiv {
        return;
    }

    let world_pos = get_world_pos(idx);

    for (var i = 0u; i < in_tile_info.commands_count; i++) {
        let cmd  = in_commands[i];
        let dist = distance(world_pos, cmd.world_position);
        if dist >= cmd.radius {
            continue;
        }

        // Linear distance falloff — strength is the maximum effect at brush center.
        let effect = (1.0 - dist / cmd.radius) * cmd.strength;

        if cmd.brush_type == 0.0 {
            // Paint: blend toward the brush color weighted by falloff.
            let cur = textureLoad(in_splatmap, vec2<i32>(idx), layer);
            textureStore(in_splatmap, vec2<i32>(idx), layer, mix(cur, cmd.color, effect));
        } else if cmd.brush_type == 1.0 {
            // Erase: blend toward the base material [1,0,0,0].
            let cur = textureLoad(in_splatmap, vec2<i32>(idx), layer);
            textureStore(in_splatmap, vec2<i32>(idx), layer,
                mix(cur, vec4<f32>(1.0, 0.0, 0.0, 0.0), effect));
        } else if cmd.brush_type == 2.0 {
            // Raise: add a small delta to the heightmap.
            let h = textureLoad(in_heightmap, vec2<i32>(idx), layer).r;
            textureStore(in_heightmap, vec2<i32>(idx), layer,
                vec4<f32>(h + effect * 0.01, 0.0, 0.0, 1.0));
        } else if cmd.brush_type == 3.0 {
            // Lower: subtract a small delta from the heightmap.
            let h = textureLoad(in_heightmap, vec2<i32>(idx), layer).r;
            textureStore(in_heightmap, vec2<i32>(idx), layer,
                vec4<f32>(h - effect * 0.01, 0.0, 0.0, 1.0));
        } else if cmd.brush_type == 4.0 {
            // Smooth: blend toward a 3x3 box-average of neighbors.
            let h   = textureLoad(in_heightmap, vec2<i32>(idx), layer).r;
            var sum = 0.0;
            for (var dy: i32 = -1; dy <= 1; dy++) {
                for (var dx: i32 = -1; dx <= 1; dx++) {
                    sum += textureLoad(
                        in_heightmap, vec2<i32>(idx) + vec2<i32>(dx, dy), layer).r;
                }
            }
            textureStore(in_heightmap, vec2<i32>(idx), layer,
                vec4<f32>(mix(h, sum / 9.0, effect), 0.0, 0.0, 1.0));
        } else if cmd.brush_type == 5.0 {
            // Flatten: blend toward the click-point height.
            let h = textureLoad(in_heightmap, vec2<i32>(idx), layer).r;
            textureStore(in_heightmap, vec2<i32>(idx), layer,
                vec4<f32>(mix(h, cmd.target_height, effect), 0.0, 0.0, 1.0));
        }
    }
}

fn get_world_pos(idx: vec2<u32>) -> vec2<f32> {
    let s = in_tile_info.tile_subdivisions;
    let t = in_tile_info.tile_size;
    let p = in_tile_info.tile_idx;
    return vec2<f32>(
        p.x * t.x - t.x * 0.5 + f32(idx.x) * (t.x / s),
        p.y * t.y - t.y * 0.5 + f32(idx.y) * (t.y / s)
    );
}
