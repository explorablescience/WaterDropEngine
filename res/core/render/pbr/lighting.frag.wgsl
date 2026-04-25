struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>
};

// Camera uniform buffer
struct Camera {
    world_to_view: mat4x4<f32>,
    view_to_ndc: mat4x4<f32>,
    ndc_to_view: mat4x4<f32>,
    view_to_world: mat4x4<f32>,
    position: vec4<f32>
}
@group(0) @binding(0) var<uniform> in_camera: Camera;

// G-Buffer textures
// Depth texture: Linear view-space depth, stored in R32Float
@group(1) @binding(0) var in_depth_texture: texture_2d<f32>;
@group(1) @binding(1) var in_depth_sampler: sampler;
@group(1) @binding(2) var in_albedo_metallic_t:  texture_2d<f32>;
@group(1) @binding(3) var in_albedo_metallic_s:  sampler;
@group(1) @binding(4) var in_normal_roughness_t: texture_2d<f32>;
@group(1) @binding(5) var in_normal_roughness_s: sampler;

// Light structure and storage buffer
struct Light {
    /// (x, y, z) = World space position of the light - w = Light intensity (brightness in lumens)
    position_intensity: vec4<f32>,
    /// (x, y, z) = Light color - w = Light type (0 = directional, 1 = point, 2 = spot)
    color_type: vec4<f32>,
    /// (x, y, z) = Direction of the light - w = Light range (max influence distance)
    direction_range: vec4<f32>,
    /// Spot light inner and outer cone angles (cos values). Only used for spot lights.
    spot_cone: vec4<f32>
};
@group(2) @binding(0) var<storage, read> in_lights: array<Light>;




// ======== HELPER FUNCTIONS ========
/// Reconstruct world position from screen coordinates and linear view-space depth
fn world_from_screen_coord_depth(uv: vec2<f32>, view_z: f32) -> vec3<f32> {
    // Reconstruct NDC position
    let ndc_x = uv.x * 2.0 - 1.0;
    let ndc_y = (1.0 - uv.y) * 2.0 - 1.0; // Flip Y for NDC
    let ndc_pos = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    
    // Reconstruct view space position
    let view_pos4 = in_camera.ndc_to_view * ndc_pos;
    let view_pos = view_pos4.xyz / view_pos4.w;
    let view_dir = normalize(view_pos);
    let view_position = view_dir * (view_z / -view_dir.z); // scale so view_position.z == -view_z
    
    // Transform to world space
    let world_pos4 = in_camera.view_to_world * vec4<f32>(view_position, 1.0);
    return world_pos4.xyz / world_pos4.w;
}



// ======= LIGHTING HELPERS =======
struct LightData {
    light_dir: vec3<f32>,
    radiance:  vec3<f32>
};

/// Get light direction and radiance based on light type and fragment position
fn get_light_data(light: Light, frag_world_position: vec3<f32>) -> LightData {
    // Initialize variables
    var light_dir: vec3<f32>;
    var radiance: vec3<f32>;

    // Get the type of light (0 = directional, 1 = point, 2 = spot)
    let light_type = i32(light.color_type.w);
    let light_intensity = light.position_intensity.w;
    let light_color = light.color_type.rgb;
        
    // Determine light direction and radiance
    if light_type == 0 { // Directional light
        light_dir = normalize(-light.direction_range.xyz);
        radiance = light_color * light_intensity;
    }
    else if light_type == 1 { // Point light
        let light_to_frag = light.position_intensity.xyz - frag_world_position;
        let distance = length(light_to_frag);
        light_dir = normalize(light_to_frag);
        let light_range = light.direction_range.w;
        
        // Smooth distance falloff: inverse-square-law with smooth cutoff at range
        let attenuation = 1.0 / max(distance * distance, 0.01); // Prevent division by zero at close range
        let range_falloff = smoothstep(light_range, 0.0, distance); // Smooth cutoff at light range
        
        radiance = light_color * light_intensity * attenuation * range_falloff;
    }
    else if light_type == 2 { // Spot light
        let light_to_frag = light.position_intensity.xyz - frag_world_position;
        let distance = length(light_to_frag);
        light_dir = normalize(light_to_frag);
        let light_range = light.direction_range.w;
        
        // Smooth distance falloff: inverse-square-law with smooth cutoff at range
        let attenuation = 1.0 / max(distance * distance, 0.01);
        let range_falloff = smoothstep(light_range, 0.0, distance);
        
        // Spot light cone intensity
        let theta = dot(light_dir, normalize(-light.direction_range.xyz));
        let inner_cos = light.spot_cone.x; // cos of inner angle
        let outer_cos = light.spot_cone.y; // cos of outer angle
        let cone_intensity = smoothstep(outer_cos, inner_cos, theta); // Smooth cone falloff
        
        radiance = light_color * light_intensity * attenuation * range_falloff * cone_intensity;
    }
    else {
        // Unknown light type
        light_dir = vec3<f32>(0.0, 0.0, 0.0);
        radiance = vec3<f32>(1.0, 0.0, 1.0); // Magenta to indicate error
    }
    return LightData(light_dir, radiance);
}



// ========= BRDF FUNCTIONS =========
const PI: f32 = 3.14159265359;

/// Trowbridge-Reitz GGX normal distribution function.
/// This function gives the distribution of microfacets on the surface, that is the D coefficient.
fn distribution_ggx(n: vec3<f32>, h: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a; // Use roughness squared for better visual results (Disney principled)
    let n_dot_h = max(dot(n, h), 0.0);
    let n_dot_h2 = n_dot_h * n_dot_h;
    
    let num = a2;
    var denom = n_dot_h2 * (a2 - 1.0) + 1.0;
    denom = PI * denom * denom;
    return num / denom;
}

/// Schlick-GGX geometry function.
fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;

    let num = n_dot_v;
    let denom = n_dot_v * (1.0 - k) + k;
    return num / denom;
}

/// Smith's method combining geometry obstruction and shadowing.
/// This function gives the amount of light blocked based on the angles between normal, view direction, and light direction, that is the G coefficient.
fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let ggx1 = geometry_schlick_ggx(n_dot_l, roughness);
    let ggx2 = geometry_schlick_ggx(n_dot_v, roughness);
    return ggx1 * ggx2;
}

/// Fresnel-Schlick approximation (theta is the angle between view direction and half vector)
/// This function gives the amount of light reflected based on the viewing angle, that is the F coefficient.
fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

struct SpecularFresnel {
    specular: vec3<f32>,
    fresnel:  vec3<f32>
};

/// Cook-Torrance specular BRDF calculation.
/// Returns the specular reflection color and fresnel term.
fn specular_cook_torrance(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, roughness: f32, f0: vec3<f32>) -> SpecularFresnel {
    // Dot products
    let n_dot_v = max(dot(n, v), 0.0);
    let n_dot_l = max(dot(n, l), 0.0);

    // Half vector
    let h = normalize(v + l);
    let h_dot_v = max(dot(h, v), 0.0);
    
    // Cook-Torrance BRDF components
    let d = distribution_ggx(n, h, roughness);
    let f = fresnel_schlick(h_dot_v, f0);
    let g = geometry_smith(n_dot_v, n_dot_l, roughness);
    
    // Final specular term (+ epsilon to prevent division by zero)
    let num = d * f * g;
    let denom = 4.0 * n_dot_v * n_dot_l + 0.0001;
    let specular = num / denom;
    
    // Return both specular color and fresnel term
    return SpecularFresnel(specular, f);
}

/// Complete BRDF calculation for a given light source.
/// Here, n is the normal, v is the view direction, l is the light direction,
/// albedo is the base color, metallic is the metallic factor, roughness is the surface roughness, and f0 is the base reflectivity.
/// Returns the final color contribution from this light.
fn brdf_for_light(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, albedo: vec3<f32>, metallic: f32, roughness: f32, f0: vec3<f32>) -> vec3<f32> {
    // Get specular component
    let specular_fresnel = specular_cook_torrance(n, v, l, roughness, f0);
    let specular = specular_fresnel.specular;
    let f = specular_fresnel.fresnel;
        
    // Compute refracted and reflected light ratios
    let ks = f; // Fresnel represents kS as it describes the amount of reflected light
    var kd = vec3<f32>(1.0) - ks; // kd = 1 - ks (energy conservation)
    kd = kd * (1.0 - metallic);   // Metals have no diffuse component
    
    // Return the brdf result
    return kd * albedo / PI + specular;
}



// ======== MAIN FRAGMENT SHADER ========
@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Reconstruct world position from linear view-space depth
    let view_z = textureSample(in_depth_texture, in_depth_sampler, in.tex_coord).r;
    let world_position = world_from_screen_coord_depth(in.tex_coord, view_z);

    // Render background color if depth is at far plane (no geometry)
    if view_z <= 0.0 {
        discard;
    }

    // Read G-Buffer
    let tmp_g_albedo_metallic  = textureSample(in_albedo_metallic_t, in_albedo_metallic_s, in.tex_coord);
    let tmp_g_normal_roughness = textureSample(in_normal_roughness_t, in_normal_roughness_s, in.tex_coord);
    let albedo    = tmp_g_albedo_metallic.rgb;
    let metallic  = tmp_g_albedo_metallic.a;
    let normal    = normalize(tmp_g_normal_roughness.xyz);
    let roughness = tmp_g_normal_roughness.a;

    // Get view direction from camera to fragment
    let view_dir = normalize(in_camera.position.xyz - world_position);

    // Calculate base reflectivity (F0)
    // For dielectrics, F0 is around 0.04, for metals it's the albedo color
    var f0 = vec3<f32>(0.04);
    f0 = mix(f0, albedo, metallic);

    // Compute lighting for each light
    var lo = vec3<f32>(0.0);
    let lights_count = i32(arrayLength(&in_lights));
    for (var i = 0; i < lights_count; i = i + 1) {
        // Get light direction and radiance at fragment position
        let light = in_lights[i];
        let light_data = get_light_data(light, world_position);
        let light_dir = light_data.light_dir;
        let radiance = light_data.radiance;
        
        // Calculate BRDF for this light and accumulate
        let n_dot_l = max(dot(normal, light_dir), 0.0);
        let brdf = brdf_for_light(normal, view_dir, light_dir, albedo, metallic, roughness, f0);
        lo += brdf * radiance * n_dot_l;
    }

    // Ambient lighting (use IBL as ambient term)
    let n_dot_v = max(dot(normal, view_dir), 0.0);
    let ks = fresnel_schlick(n_dot_v, f0);
    var kd = 1.0 - ks;
    kd = kd * (1.0 - metallic);
    let irradiance = vec3<f32>(1.0) * 0.001; // Placeholder ambient term (could be replaced with IBL cubemap)
    let diffuse = irradiance * albedo;
    let ambient = kd * diffuse; // Note: * ambiant_occlusion (not implemented here)

    // Final color before tone mapping
    var color = lo + ambient;

    // HDR tone mapping (Reinhard)
    color = color / (color + vec3<f32>(1.0));

    // Return the final color
    return vec4<f32>(color, 1.0);
}
