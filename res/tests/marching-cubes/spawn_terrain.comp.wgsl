// Description of the chunks parameters
struct MarchingCubesDescription {
    translation:       vec4<f32>,   // Translation in world space of the current chunk (x, y, z, 0)
    chunk_length:      vec4<f32>,   // Length of the chunk
    chunk_sub_count:   vec4<u32>,   // Number of sub-chunks
    triangles_counter: u32,         // Counter of the triangles
    iso_level:         f32,         // Iso level
    padding:           vec2<f32>    // Padding
}
@group(0) @binding(0) var<storage> in_desc: MarchingCubesDescription;

// List of points of the marching cubes values
@group(1) @binding(0) var<storage, read_write> in_points: array<vec4<f32>>; // Points of the marching cubes (x, y, z, f(x, y, z))

// Terrain noise parameters
struct TerrainNoiseParameters {
    amplitude:      f32,   // Amplitude of the noise
    frequency:      f32,   // Frequency of the noise
    ground_percent: f32,   // Percentage of the ground
    octaves:        u32,   // Number of octaves
    persistence:    f32,   // Persistence of the noise
    lacunarity:     f32    // Lacunarity of the noise
}
@group(2) @binding(0) var<uniform> in_noise: TerrainNoiseParameters;


@compute @workgroup_size(10, 10, 10)
fn main(@builtin(global_invocation_id) thread_id: vec3<u32>) {
    // Check if out of bounds
    if thread_id.x >= in_desc.chunk_sub_count.x || thread_id.y >= in_desc.chunk_sub_count.y || thread_id.z >= in_desc.chunk_sub_count.z {
        return;
    }

    // Get the position of the current point
    let position = vec3<f32>(
        in_desc.translation.x - in_desc.chunk_length.x / 2.0 + f32(thread_id.x) * in_desc.chunk_length.x / (f32(in_desc.chunk_sub_count.x) - 1.0),
        in_desc.translation.y - in_desc.chunk_length.y / 2.0 + f32(thread_id.y) * in_desc.chunk_length.y / (f32(in_desc.chunk_sub_count.y) - 1.0),
        in_desc.translation.z - in_desc.chunk_length.z / 2.0 + f32(thread_id.z) * in_desc.chunk_length.z / (f32(in_desc.chunk_sub_count.z) - 1.0)
    );

    // Generate the noise
    var height = 0.0;
    var amplitude = in_noise.amplitude;
    var frequency = in_noise.frequency;
    for (var i = 0; i < i32(in_noise.octaves); i = i + 1) {
        height = height + amplitude * simplex_noise(position * frequency);
        amplitude = amplitude * in_noise.persistence;
        frequency = frequency * in_noise.lacunarity;
    }
    let ground = position.y + in_noise.ground_percent * in_desc.chunk_length.y;
    let value = ground + height;

    // Store the value
    in_points[index_from_coord(thread_id, vec3<u32>(in_desc.chunk_sub_count.xyz))] = vec4<f32>(position, value);
}

/**
 * Get the index of a point in the grid from its coordinates.
 * 
 * # Arguments
 * 
 * * `x` - The x coordinate.
 * * `y` - The y coordinate.
 * * `z` - The z coordinate.
 * * `chunk_sub_count` - The number of grid cells in each dimension.
 */
fn index_from_coord(coord: vec3<u32>, chunk_sub_count: vec3<u32>) -> u32 {
    return (coord.x * chunk_sub_count.y + coord.y) * chunk_sub_count.z + coord.z;
}

/**
* Generate simplex noise from a 3D position.
* Based on https://github.com/ashima/webgl-noise.
* 
* # Arguments
* 
* * `position` - The position in 3D space.
*/
fn simplex_noise(v: vec3<f32>) -> f32 {
    let C = vec2<f32>(1.0 / 6.0, 1.0 / 3.0);
    let D = vec4<f32>(0.0, 0.5, 1.0, 2.0);

    // First corner
    var i = floor(v + dot(v, C.yyy));
    let x0 = v - i + dot(i, C.xxx);

    // Other corners
    let g = step(x0.yzx, x0.xyz);
    let l = 1.0 - g;
    let i1 = min(g.xyz, l.zxy);
    let i2 = max(g.xyz, l.zxy);

    let x1 = x0 - i1 + C.xxx;
    let x2 = x0 - i2 + C.yyy;
    let x3 = x0 - D.yyy;

    // Permutations
    i = mod289_3(i);
    let p = permute(permute(permute(
        i.z + vec4<f32>(0.0, i1.z, i2.z, 1.0))
        + i.y + vec4<f32>(0.0, i1.y, i2.y, 1.0))
        + i.x + vec4<f32>(0.0, i1.x, i2.x, 1.0));

    // Gradients
    let n_ = 0.142857142857; // 1.0/7.0
    let ns = n_ * D.wyz - D.xzx;

    let j = p - 49.0 * floor(p * ns.z * ns.z);

    let x_ = floor(j * ns.z);
    let y_ = floor(j - 7.0 * x_);

    let x = x_ * ns.x + ns.yyyy;
    let y = y_ * ns.x + ns.yyyy;
    let h = 1.0 - abs(x) - abs(y);

    let b0 = vec4<f32>(x.xy, y.xy);
    let b1 = vec4<f32>(x.zw, y.zw);

    let s0 = floor(b0) * 2.0 + 1.0;
    let s1 = floor(b1) * 2.0 + 1.0;
    let sh = -step(h, vec4<f32>(0.0));

    let a0 = b0.xzyw + s0.xzyw * sh.xxyy;
    let a1 = b1.xzyw + s1.xzyw * sh.zzww;

    var p0 = vec3<f32>(a0.xy, h.x);
    var p1 = vec3<f32>(a0.zw, h.y);
    var p2 = vec3<f32>(a1.xy, h.z);
    var p3 = vec3<f32>(a1.zw, h.w);

    // Normalize gradients
    let norm = taylorInvSqrt(vec4<f32>(dot(p0, p0), dot(p1, p1), dot(p2, p2), dot(p3, p3)));
    p0 *= norm.x;
    p1 *= norm.y;
    p2 *= norm.z;
    p3 *= norm.w;

    // Mix final noise value
    let m = max(0.5 - vec4<f32>(dot(x0, x0), dot(x1, x1), dot(x2, x2), dot(x3, x3)), vec4<f32>(0.0));
    let m2 = m * m;
    return 105.0 * dot(m2 * m2, vec4<f32>(dot(p0, x0), dot(p1, x1), dot(p2, x2), dot(p3, x3)));
}

fn mod289_3(x: vec3<f32>) -> vec3<f32> {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

fn mod289_4(x: vec4<f32>) -> vec4<f32> {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

fn permute(x: vec4<f32>) -> vec4<f32> {
    return mod289_4(((x * 34.0) + 1.0) * x);
}

fn taylorInvSqrt(r: vec4<f32>) -> vec4<f32> {
    return 1.79284291400159 - 0.85373472095314 * r;
}
