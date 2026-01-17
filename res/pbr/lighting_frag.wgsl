struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>
};

// Camera uniform buffer
struct Camera {
    world_to_ndc: mat4x4<f32>,
    ndc_to_world: mat4x4<f32>,
    position: vec4<f32>
}
@group(0) @binding(0) var<uniform> in_camera: Camera;

// G-Buffer textures
// Depth texture: NDC space (z/w), range [0, 1], stored in R32Float
@group(1) @binding(0) var in_depth_texture: texture_2d<f32>;
@group(1) @binding(1) var in_depth_sampler: sampler;
@group(1) @binding(2) var in_albedo_metallic_t:  texture_2d<f32>;
@group(1) @binding(3) var in_albedo_metallic_s:  sampler;
@group(1) @binding(4) var in_normal_roughness_t: texture_2d<f32>;
@group(1) @binding(5) var in_normal_roughness_s: sampler;

// Light structure and storage buffer
struct Light {
    /// (x, y, z) = World space position of the light - w = Number of lights (for the first light only)
    position_number:    vec4<f32>,
    /// (x, y, z) = Direction of the light - w = Type of the light (0 = directional, 1 = point, 2 = spot)
    direction_type:     vec4<f32>,
    /// (x, y, z) = Ambient color of the light - w = Constant attenuation factor if the light is a point light. It is the cos of the inner cut-off angle in radians if the light is a spot light.
    ambient_att_cst:    vec4<f32>,
    /// (x, y, z) = Diffuse color of the light - w = Linear attenuation factor if the light is a point light. It is the cos of the outer cut-off angle in radians if the light is a spot light.
    diffuse_att_linear: vec4<f32>,
    /// (x, y, z) = Specular color of the light - w = Quadratic attenuation factor if the light is a point light.
    specular_att_quadr: vec4<f32>,
    /// Inner and outer cut-off angles in radians if the light is a spot light.
    cut_off: vec4<f32>
};
@group(2) @binding(0) var<storage> in_lights: array<Light>;




// ======== HELPER FUNCTIONS ========
/// Convert screen coordinates and depth to world space position
fn world_from_screen_coord_depth(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    // NDC position : Need to flip Y coordinate, and map depth from [0, 1] to [-1, 1]
    let ndc_position = vec4<f32>(uv.x * 2.0 - 1.0, (1 - uv.y) * 2.0 - 1.0, depth, 1.0);

    // Retrieve world position from NDC, knowing camera world position and matrices
    let view_position  = in_camera.ndc_to_world * ndc_position;
    let world_position = view_position.xyz / view_position.w;
    return world_position;
}



// ======= LIGHTING HELPERS =======
struct LightData {
    light_dir: vec3<f32>,
    radiance:  vec3<f32>
};

/// Get light direction and radiance based on light type and fragment position
fn get_light_data(light: Light, position: vec3<f32>) -> LightData {
    // Initialize variables
    var light_dir: vec3<f32>;
    var radiance: vec3<f32>;
    var attenuation: f32 = 1.0;

    // Get the type of light (0 = directional, 1 = point, 2 = spot)
    let light_type = i32(light.direction_type.w);
        
    // Determine light direction and radiance
    if light_type == 0 { // Directional light
        light_dir = normalize(-light.direction_type.xyz);
        radiance = light.diffuse_att_linear.rgb;
    }
    else if light_type == 1 { // Point light
        let light_to_frag = light.position_number.xyz - position;
        let distance = length(light_to_frag);
        light_dir = normalize(light_to_frag);
        
        // Attenuation for point light
        attenuation = 1.0 / (light.ambient_att_cst.w 
            + light.diffuse_att_linear.w * distance 
            + light.specular_att_quadr.w * distance * distance);
        
        radiance = light.diffuse_att_linear.rgb * attenuation;
    }
    else if light_type == 2 { // Spot light
        let light_to_frag = light.position_number.xyz - position;
        let distance = length(light_to_frag);
        light_dir = normalize(light_to_frag);
        
        // Attenuation for spot light
        attenuation = 1.0 / (light.ambient_att_cst.w 
            + light.diffuse_att_linear.w * distance 
            + light.specular_att_quadr.w * distance * distance);
        
        // Spot light cone intensity
        let theta = dot(light_dir, normalize(-light.direction_type.xyz));
        let inner_cutoff = light.ambient_att_cst.w; // cos of inner angle
        let outer_cutoff = light.diffuse_att_linear.w; // cos of outer angle
        let epsilon = inner_cutoff - outer_cutoff;
        let intensity = clamp((theta - outer_cutoff) / epsilon, 0.0, 1.0);
        
        radiance = light.diffuse_att_linear.rgb * attenuation * intensity;
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
    var denom = (n_dot_h2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;
    return num / denom;
}

/// Schlick-GGX geometry function.
fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return n_dot_v / (n_dot_v * (1.0 - k) + k);
}

/// Smith's method combining geometry obstruction and shadowing.
/// This function gives the amount of light blocked based on the angles between normal, view direction, and light direction, that is the G coefficient.
fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let ggx2 = geometry_schlick_ggx(n_dot_v, roughness);
    let ggx1 = geometry_schlick_ggx(n_dot_l, roughness);
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
    
    // Cook-Torrance BRDF components
    let d = distribution_ggx(n, h, roughness);
    let f = fresnel_schlick(max(dot(h, v), 0.0), f0);
    let g = geometry_smith(n_dot_v, n_dot_l, roughness);
    
    // Final specular term (+ epsilon to prevent division by zero)
    let specular = d * f * g / (4.0 * n_dot_v * n_dot_l + 0.0001);
    
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
    let metallic_factor = mix(1.0, 0.0, metallic); // Invert metallic for kd calculation (Metallic surfaces don't have diffuse)
    let kd = (vec3<f32>(1.0) - ks) * metallic_factor; // kd = 1 - ks (energy conservation)
    
    // Return the brdf result
    return kd * albedo / PI + specular;
}



// ======== MAIN FRAGMENT SHADER ========
@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Read world position of the object in world space from depth buffer (interval [0, 1])
    let depth = textureSample(in_depth_texture, in_depth_sampler, in.tex_coord).r;
    if depth < 0.01 { // Discard background
        discard;
    }
    let position = world_from_screen_coord_depth(in.tex_coord, depth);

    // Read G-Buffer
    let tmp_g_albedo_metallic  = textureSample(in_albedo_metallic_t, in_albedo_metallic_s, in.tex_coord);
    let tmp_g_normal_roughness = textureSample(in_normal_roughness_t, in_normal_roughness_s, in.tex_coord);
    let albedo    = tmp_g_albedo_metallic.rgb;
    let metallic  = tmp_g_albedo_metallic.a;
    let normal    = normalize(tmp_g_normal_roughness.xyz);
    let roughness = tmp_g_normal_roughness.a;

    // Get view direction from camera to fragment
    let view_dir = normalize(in_camera.position.xyz - position);

    // Calculate base reflectivity (F0)
    // For dielectrics, F0 is around 0.04, for metals it's the albedo color
    var f0 = vec3<f32>(0.04);
    f0 = mix(f0, albedo, metallic);

    // Get number of lights
    let lights_count = i32(in_lights[0].position_number.w);

    // Ambient lighting (simple IBL approximation)
    let ambient_strength = 0.03;
    var lo = albedo * ambient_strength;

    // Compute lighting for each light
    for (var i = 0; i < lights_count; i = i + 1) {
        // Get light direction and radiance at fragment position
        let light = in_lights[i];
        let light_data = get_light_data(light, position);
        let light_dir = light_data.light_dir;
        let radiance = light_data.radiance;
        
        // Calculate BRDF for this light and accumulate
        let n_dot_l = max(dot(normal, light_dir), 0.0);
        let brdf = brdf_for_light(normal, view_dir, light_dir, albedo, metallic, roughness, f0);
        lo += brdf * radiance * n_dot_l;
    }

    // HDR tone mapping (simple Reinhard)
    let hdr_lo = lo / (lo + vec3<f32>(1.0));

    // Return the final color
    return vec4<f32>(hdr_lo, 1.0);
}
