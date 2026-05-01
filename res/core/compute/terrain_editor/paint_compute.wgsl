struct TileInfo {
    tile_idx:         vec2<f32>,
    tile_size:        vec2<f32>,
    tile_subdivisions: f32,
    tile_layer:       u32
}
var<push_constant> in_tile_info: TileInfo;

struct Command {
    world_position: vec2<f32>,
    radius:         f32,
    strength:       f32,
    color:          vec4<f32>,
    brush_type:     f32,  // 0=Paint 1=Erase 2=Raise 3=Lower 4=Smooth 5=Flatten
    _padding:       vec3<f32>
}
@group(0) @binding(0) var<storage, read> in_commands: array<Command>;

@group(1) @binding(0) var in_heightmap: texture_storage_2d_array<r8unorm,    read_write>;
@group(1) @binding(1) var in_splatmap:  texture_storage_2d_array<rgba8unorm, read_write>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let tile_subdiv  = u32(in_tile_info.tile_subdivisions);
    let layer        = i32(in_tile_info.tile_layer);
    let command_index = global_id.x / (tile_subdiv * tile_subdiv);
    let pixel_index   = global_id.x % (tile_subdiv * tile_subdiv);
    let idx           = vec2<u32>(pixel_index % tile_subdiv, pixel_index / tile_subdiv);
    let world_pos     = get_world_pos(idx);

    if (idx.x >= tile_subdiv || idx.y >= tile_subdiv) {
        return;
    }
    if (command_index >= arrayLength(&in_commands)) {
        return;
    }

    let cmd      = in_commands[command_index];
    let distance = distance(world_pos, cmd.world_position);
    if (distance >= cmd.radius) {
        return;
    }

    let strength = cmd.strength * 0.01;
    let effect   = (1.0 - distance / cmd.radius) * strength;

    // Paint / Erase affect the splatmap.
    if (cmd.brush_type == 0.0) {
        let current = textureLoad(in_splatmap, vec2<i32>(idx), layer);
        textureStore(in_splatmap, vec2<i32>(idx), layer, mix(current, cmd.color, cmd.strength));
    } else if (cmd.brush_type == 1.0) {
        textureStore(in_splatmap, vec2<i32>(idx), layer, vec4<f32>(0.0, 0.0, 0.0, 1.0));
    }

    // Raise / Lower / Smooth / Flatten affect the heightmap.
    if (cmd.brush_type == 2.0) {
        let h = textureLoad(in_heightmap, vec2<i32>(idx), layer).r;
        textureStore(in_heightmap, vec2<i32>(idx), layer, vec4<f32>(h + effect, 0.0, 0.0, 1.0));
    } else if (cmd.brush_type == 3.0) {
        let h = textureLoad(in_heightmap, vec2<i32>(idx), layer).r;
        textureStore(in_heightmap, vec2<i32>(idx), layer, vec4<f32>(h - effect, 0.0, 0.0, 1.0));
    } else if (cmd.brush_type == 4.0) {
        let h = textureLoad(in_heightmap, vec2<i32>(idx), layer).r;
        var sum = 0.0;
        for (var dy: i32 = -1; dy <= 1; dy++) {
            for (var dx: i32 = -1; dx <= 1; dx++) {
                sum += textureLoad(in_heightmap, vec2<i32>(idx) + vec2<i32>(dx, dy), layer).r;
            }
        }
        textureStore(in_heightmap, vec2<i32>(idx), layer, vec4<f32>(mix(h, sum / 9.0, effect), 0.0, 0.0, 1.0));
    } else if (cmd.brush_type == 5.0) {
        let h      = textureLoad(in_heightmap, vec2<i32>(idx), layer).r;
        let target_pos = cmd.world_position.y;
        textureStore(in_heightmap, vec2<i32>(idx), layer, vec4<f32>(mix(h, target_pos, effect), 0.0, 0.0, 1.0));
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
