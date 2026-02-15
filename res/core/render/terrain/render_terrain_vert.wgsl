struct ModelInput {
    @location(0) position:  vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) normal:    vec3<f32>
};
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
};

// From world space to normalized device coordinates
struct Camera {
    world_to_view: mat4x4<f32>,
    view_to_ndc: mat4x4<f32>
}
@group(0) @binding(0) var<uniform> in_camera: Camera;

// Simple Perlin noise implementation
fn fade(t: f32) -> f32 {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}

fn grad(hash: u32, x: f32, y: f32) -> f32 {
    let h = hash & 3u;
    var u = x;
    if (h < 2u) {
        u = x;
    } else {
        u = y;
    }
    var v = y;
    if (h < 2u) {
        v = y;
    } else {
        v = x;
    }
    
    if (h & 1u) == 0u {
        u = -u;
    }
    if (h & 2u) == 0u {
        v = -v;
    }
    return u + v;
}

fn perlin(p: vec2<f32>) -> f32 {
    // Do not use a permutation table for simplicity, just hash the coordinates
    let xi = u32(floor(p.x)) & 255u;
    let yi = u32(floor(p.y)) & 255u;
    let xf = p.x - floor(p.x);
    let yf = p.y - floor(p.y);
    let u = fade(xf);
    let v = fade(yf);
    let aa = (xi + yi * 256u) & 255u;
    let ab = (xi + (yi + 1u) * 256u) & 255u;
    let ba = ((xi + 1u) + yi * 256u) & 255u;
    let bb = ((xi + 1u) + (yi + 1u) * 256u) & 255u;
    let x1 = mix(grad(aa, xf, yf), grad(ba, xf - 1.0, yf), u);
    let x2 = mix(grad(ab, xf, yf - 1.0), grad(bb, xf - 1.0, yf - 1.0), u);
    return (mix(x1, x2, v) + 1.0) / 2.0; // Normalize to [0, 1]
}

// Compute the height of the terrain at a given xz position
fn terrain(p: vec2<f32>) -> f32 {
    // Height function using octave Perlin noise
    let scale = 0.1;
    var height = 0.0;
    let persistence = 0.5;
    let octaves = 2;
    var amplitude = 1.2;
    var frequency = 0.5;
    for (var i = 0; i < octaves; i = i + 1) {
        height = height + perlin(p * scale * frequency) * amplitude;
        amplitude = amplitude * persistence;
        frequency = frequency * 2.0;
    }
    return height * 10.0; // Scale the height
}



@vertex
fn main(@builtin(instance_index) instance: u32, model: ModelInput) -> VertexOutput {
    var out: VertexOutput;

    let obj_to_world = mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0)
    );

    var world_pos = obj_to_world * vec4<f32>(model.position, 1.0);

    // Add some noise to the height based on the xz position
    world_pos.y += terrain(world_pos.xz);

    let view_pos4 = in_camera.world_to_view
        * world_pos;
    let view_pos = view_pos4.xyz / view_pos4.w;
    out.clip_position = in_camera.view_to_ndc * vec4<f32>(view_pos, 1.0);

    // Compute normal with finite differences
    let delta = 0.1;
    let heightL = terrain((world_pos.xz - vec2<f32>(delta, 0.0)));
    let heightR = terrain((world_pos.xz + vec2<f32>(delta, 0.0)));
    let heightD = terrain((world_pos.xz - vec2<f32>(0.0, delta)));
    let heightU = terrain((world_pos.xz + vec2<f32>(0.0, delta)));
    let normal = normalize(vec3<f32>(heightL - heightR, 2.0 * delta, heightD - heightU));
    out.normal = normal;

    return out;
}
