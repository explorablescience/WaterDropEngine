struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) @interpolate(flat) normal: vec3<f32>
};

// From world space to normalized device coordinates
struct Camera {
    world_to_ndc: mat4x4<f32>,
    ndc_to_world: mat4x4<f32>,
    position: vec4<f32>
}
@group(0) @binding(0) var<uniform> in_camera: Camera;

// Light structure
struct Light {
    /// World space position of the directional light for xyz. If it is the first element, the w component is the number of lights.
    position_number: vec4<f32>,
    /// World space direction of the light. The w component is the type of the light: 0 for directional, 1 for point, 2 for spot.
    direction_type:  vec4<f32>,
    /// Ambient color of the light. The w component is the constant attenuation factor if the light is a point light. It is the cos of the inner cut-off angle in radians if the light is a spot light.
    ambient_const:   vec4<f32>,
    /// Diffuse color of the light. The w component is the linear attenuation factor if the light is a point light. It is the cos of the outer cut-off angle in radians if the light is a spot light.
    diffuse_linea:   vec4<f32>,
    /// Specular color of the light. The w component is the quadratic attenuation factor if the light is a point light.
    specular_quadr:  vec4<f32>,
    /// Inner and outer cut-off angles in radians if the light is a spot light.
    cut_off:         vec4<f32>
};
@group(1) @binding(0) var<storage> in_lights: array<Light>;

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Constants for the material
    let albedo    = vec3<f32>(0.2, 0.2, 0.8);
    let specular  = vec3<f32>(0.3, 0.3, 0.3);
    let shininess = 32.0;
    let view_dir  = normalize(in_camera.position.xyz - in.position);

    // Compute lighting
    let lights_count = i32(in_lights[0].position_number.w);
    var transmitted = pow(vec3<f32>(0.1), vec3<f32>(2.2));
    for (var i = 0; i < lights_count; i = i + 1) {
        let light = in_lights[i];
        let light_type = i32(light.direction_type.w);

        // Light direction
        var light_dir = -normalize(vec3<f32>(-0.1, 0.8, -0.2));
        if light_type == 0 { // Directional light
            light_dir = -normalize(light.direction_type.xyz);
        }
        else if light_type == 1 || light_type == 2 { // Point light or spot light
            light_dir = normalize(light.position_number.xyz - in.position);
        }
        else {
            // Error
            return vec4<f32>(1.0, 0.0, 1.0, 1.0);
        }

        // Diffused
        let light_angle = max(dot(in.normal, light_dir), 0.0);

        // Specular
        let halfway_dir = normalize(light_dir + view_dir);
        let spec_value  = pow(max(dot(in.normal, halfway_dir), 0.0), shininess);

        // Combine results
        let ambient  =  albedo                 * light.ambient_const.rgb;
        var diffused = (albedo * light_angle)  * light.diffuse_linea.rgb;
        var specular = (specular * spec_value) * light.specular_quadr.rgb;

        // Point light or spot light
        if light_type == 1 || light_type == 2 {
            // Attenuation
            let distance = length(light.position_number.xyz - in.position);
            let attenuation = 1.0 / (light.ambient_const.w
                + light.diffuse_linea.w * distance
                + light.specular_quadr.w * distance * distance);

            diffused *= attenuation;
            specular *= attenuation;
        }

        // Spot light intensity
        if light_type == 2 {
            let theta     = dot(normalize(light.direction_type.xyz), light.direction_type.xyz);
            let epsilon   = light.cut_off.x - light.cut_off.y;
            let intensity = clamp((theta - light.diffuse_linea.w) / epsilon, 0.0, 1.0);

            diffused *= intensity;
            specular *= intensity;
        }
        transmitted += ambient + diffused + specular;
    }

    // Return the final color
    return vec4<f32>(transmitted, 1.0);
}
