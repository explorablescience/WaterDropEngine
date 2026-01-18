struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>
};

// Material description
struct PbrMaterial {
    color: vec4<f32>, // Gizmo color
};
@group(2) @binding(0) var<uniform> in_material: PbrMaterial;

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in_material.color;
}
