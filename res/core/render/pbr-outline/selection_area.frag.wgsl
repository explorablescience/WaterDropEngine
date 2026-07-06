struct SelectionAreaMaterial {
    color: vec4<f32>,
};
@group(3) @binding(0) var<uniform> in_selection_area_material: SelectionAreaMaterial;

@fragment
fn main() -> @location(0) vec4<f32> {
    return in_selection_area_material.color;
}
