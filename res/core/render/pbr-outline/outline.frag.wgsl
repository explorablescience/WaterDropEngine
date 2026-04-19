struct OutlineMaterial {
    color: vec4<f32>,
    thickness: vec4<f32>,
};
@group(3) @binding(0) var<uniform> in_outline_material: OutlineMaterial;

@fragment
fn main() -> @location(0) vec4<f32> {
    return in_outline_material.color;
}
