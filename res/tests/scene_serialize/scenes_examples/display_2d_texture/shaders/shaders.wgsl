// Resource Bindings
@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sam: sampler;

// Vertex Input and Output Structures
struct ModelInput {
    @location(0) pos: vec3<f32>,
    @location(1) uv:  vec2<f32>,
    @location(2) normal: vec3<f32>
};
struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>
};

// Vertex Shader
@vertex
fn vert(@builtin(instance_index) instance: u32, model: ModelInput) -> VertexOutput {
    var out: VertexOutput;

    out.clip = vec4<f32>(model.pos, 1.0);
    out.uv = model.uv;

    return out;
}

// Fragment Shader
@fragment
fn frag(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
    var val = textureSample(image, image_sam, uv).rgb;

    return vec4<f32>(val, 1.0);
}

