// Informations on the current processed tile
struct TileInfo {
    tile_idx: vec2<f32>,
    tile_size: vec2<f32>,
    tile_subdivisions: f32
}
var<push_constant> in_tile_info: TileInfo;

// List of commands to apply to the terrain
struct Command {
    world_position: vec2<f32>,
    radius: f32,
    strength: f32,
    color: vec4<f32>,
    brush_type: f32, // 0: Paint, 1: Erase, 2: Raise, 3: Lower, 4: Smooth, 5: Flatten
    _padding: vec3<f32>
}
@group(0) @binding(0) var<storage, read> in_commands: array<Command>;

// Terrain tile data
@group(1) @binding(0) var in_heightmap: texture_storage_2d<r8unorm, read_write>;
@group(1) @binding(1) var in_splatmap_1: texture_storage_2d<rgba8unorm, read_write>;


@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let tile_subdivisions = u32(in_tile_info.tile_subdivisions);

    // Get index
    let command_index = global_invocation_id.x / (tile_subdivisions * tile_subdivisions);
    let pixel_index = global_invocation_id.x % (tile_subdivisions * tile_subdivisions);
    let idx = vec2<u32>(pixel_index % tile_subdivisions, pixel_index / tile_subdivisions);
    let world_pos = get_world_pos(idx);

    // Check if the pixel index is within bounds
    if (idx.x >= tile_subdivisions || idx.y >= tile_subdivisions) {
        return;
    }

    // Check if the command index is within bounds
    if (command_index >= arrayLength(&in_commands)) {
        return;
    }
    let command = in_commands[command_index];

    // Set brush effect
    let command_world_pos = command.world_position;
    let distance = distance(world_pos, command_world_pos);
    if (distance < command.radius) {
        // PAINT BRUSHES
        if (command.brush_type == 0.0) { // Paint
            let current_color = textureLoad(in_splatmap_1, vec2<i32>(idx));
            let color = mix(current_color, command.color, command.strength);
            textureStore(in_splatmap_1, vec2<i32>(idx), color);
        } else if (command.brush_type == 1.0) { // Erase
            let color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
            textureStore(in_splatmap_1, vec2<i32>(idx), color);
        }

        // HEIGHT BRUSHES
        var strength = command.strength * 0.01;
        let height_effect = (1.0 - (distance / command.radius)) * strength;
        if (command.brush_type == 2.0) { // Raise
            let current_height = textureLoad(in_heightmap, vec2<i32>(idx)).r;
            let new_height = current_height + height_effect;
            textureStore(in_heightmap, vec2<i32>(idx), vec4<f32>(new_height, 0.0, 0.0, 1.0));
        } else if (command.brush_type == 3.0) { // Lower
            let current_height = textureLoad(in_heightmap, vec2<i32>(idx)).r;
            let new_height = current_height - height_effect;
            textureStore(in_heightmap, vec2<i32>(idx), vec4<f32>(new_height, 0.0, 0.0, 1.0));
        } else if (command.brush_type == 4.0) { // Smooth
            let current_height = textureLoad(in_heightmap, vec2<i32>(idx)).r;
            var sum = 0.0;
            var count = 0.0;
            for (var y: i32 = -1; y <= 1; y++) {
                for (var x: i32 = -1; x <= 1; x++) {
                    let neighbor_height = textureLoad(in_heightmap, vec2<i32>(idx) + vec2<i32>(x, y)).r;
                    sum += neighbor_height;
                    count += 1.0;
                }
            }
            let average_height = sum / count;
            let new_height = mix(current_height, average_height, height_effect);
            textureStore(in_heightmap, vec2<i32>(idx), vec4<f32>(new_height, 0.0, 0.0, 1.0));
        } else if (command.brush_type == 5.0) { // Flatten
            let current_height = textureLoad(in_heightmap, vec2<i32>(idx)).r;
            let target_height = command.world_position.y; // Use the y component of world position as target height
            let new_height = mix(current_height, target_height, height_effect);
            textureStore(in_heightmap, vec2<i32>(idx), vec4<f32>(new_height, 0.0, 0.0, 1.0));
        }
    }
}

fn get_world_pos(idx: vec2<u32>) -> vec2<f32> {
    let tile_subdivisions = in_tile_info.tile_subdivisions;
    let tile_size = in_tile_info.tile_size;
    let tile_idx = in_tile_info.tile_idx;

    return vec2<f32>(
        tile_idx.x * tile_size.x - tile_size.x / 2.0 + f32(idx.x) * (tile_size.x / tile_subdivisions),
        tile_idx.y * tile_size.y - tile_size.y / 2.0 + f32(idx.y) * (tile_size.y / tile_subdivisions)
    );
}
