// ================================================================
// Runtime parameters of the PBR lighting (see PbrParamsUniform), and its debug views.
// Include with:  #include "core/render/pbr/pbr_params.wgsl"
// ================================================================

struct PbrParams {
    /// x=direct_scale, y=ambient_scale, z=exposure, w=debug view
    lighting_params: vec4<f32>,
    /// x=metallic_scale, y=roughness_scale, z=min_roughness
    material_params: vec4<f32>
};

// Debug views, same values as PbrDebugView
const PBR_DEBUG_OFF: u32 = 0u;
const PBR_DEBUG_CLAY: u32 = 1u;
const PBR_DEBUG_DIRECT: u32 = 2u;
const PBR_DEBUG_AMBIENT_DIFFUSE: u32 = 3u;
const PBR_DEBUG_AMBIENT_SPECULAR: u32 = 4u;
const PBR_DEBUG_EMISSIVE: u32 = 5u;
const PBR_DEBUG_SHADOW_CASCADES: u32 = 6u;
const PBR_DEBUG_ALBEDO: u32 = 7u;
const PBR_DEBUG_NORMALS: u32 = 8u;
const PBR_DEBUG_ROUGHNESS: u32 = 9u;
const PBR_DEBUG_METALLIC: u32 = 10u;
const PBR_DEBUG_OCCLUSION: u32 = 11u;
const PBR_DEBUG_SHADOW: u32 = 12u;
const PBR_DEBUG_SHADOW_TEXELS: u32 = 13u;

// Linear albedo of every surface in the clay view
const PBR_DEBUG_CLAY_ALBEDO: f32 = 0.5;

fn pbr_debug_view(params: PbrParams) -> u32 {
    return u32(params.lighting_params.w);
}

// Whether the view shows a surface property rather than light (the post-processing leaves it untouched)
fn pbr_debug_is_data(view: u32) -> bool {
    return view >= PBR_DEBUG_ALBEDO;
}

// Light shown by a view, from the terms of the lit color
fn pbr_debug_light(view: u32, direct: vec3<f32>, ambient_diffuse: vec3<f32>, ambient_specular: vec3<f32>, emissive: vec3<f32>) -> vec3<f32> {
    if view == PBR_DEBUG_DIRECT {
        return direct;
    } else if view == PBR_DEBUG_AMBIENT_DIFFUSE {
        return ambient_diffuse;
    } else if view == PBR_DEBUG_AMBIENT_SPECULAR {
        return ambient_specular;
    } else if view == PBR_DEBUG_EMISSIVE {
        return emissive;
    } else if view == PBR_DEBUG_CLAY {
        return direct + ambient_diffuse + ambient_specular;
    }
    return direct + ambient_diffuse + ambient_specular + emissive;
}

// A value of a data view, displayed as it is: undoes the sRGB encoding of the LDR texture
fn pbr_debug_data(value: vec3<f32>) -> vec4<f32> {
    return vec4<f32>(pow(clamp(value, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(2.2)), 1.0);
}
