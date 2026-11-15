// Shared parameters for atmosphere shading.
// time_of_day in [0, 1]: 0/1 = midnight, 0.25 = sunrise, 0.5 = noon, 0.75 = sunset.
struct AtmosphereParams {
    // x=time_of_day, y=sun_intensity, z=twilight_softness, w=star_intensity
    time_params: vec4<f32>,
    // x=day_horizon_exp, y=night_horizon_exp, z=twilight_horizon_exp, w=twilight_intensity
    gradient_params: vec4<f32>,
    // x=sun_visibility_min, y=sun_visibility_max, z=sun_disk_min, w=sun_disk_max
    sun_params: vec4<f32>,
    // x=sun_corona_exp, y=sun_corona_intensity, z=star_density_threshold, w=star_core_scale
    sun_star_params: vec4<f32>,
    // x=star_min_altitude, y=star_max_altitude, z=ground_falloff
    star_ground_params: vec4<f32>,
    day_zenith_color: vec4<f32>,
    day_horizon_color: vec4<f32>,
    night_zenith_color: vec4<f32>,
    night_horizon_color: vec4<f32>,
    twilight_tint_color: vec4<f32>,
    sun_day_color: vec4<f32>,
    sun_night_color: vec4<f32>,
    star_color: vec4<f32>,
    ground_night_color: vec4<f32>,
    ground_day_color: vec4<f32>,
}

fn hash21(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453);
}

// Sun arc over the day.
fn sky_sun_dir(params: AtmosphereParams) -> vec3<f32> {
    let sun_elevation = (fract(params.time_params.x) - 0.25) * 6.2831853;
    let sun_azimuth = -0.95;
    return normalize(vec3<f32>(
        cos(sun_elevation) * cos(sun_azimuth),
        sin(sun_elevation),
        cos(sun_elevation) * sin(sun_azimuth)
    ));
}

// Smooth day amount in [0, 1] for the sun height.
fn sky_day(sun_height: f32, params: AtmosphereParams) -> f32 {
    let twilight_w = mix(0.06, 0.20, clamp(params.time_params.z, 0.0, 1.0));
    return smoothstep(-twilight_w, twilight_w, sun_height);
}

// Sky color gradient with the twilight tint, without the sun, the stars and the ground.
fn sky_gradient(ray: vec3<f32>, params: AtmosphereParams) -> vec3<f32> {
    let horizon = clamp(1.0 - max(ray.y, 0.0), 0.0, 1.0);
    let sun_height = sky_sun_dir(params).y;
    let twilight_w = mix(0.06, 0.20, clamp(params.time_params.z, 0.0, 1.0));
    let day = sky_day(sun_height, params);
    let twilight = 1.0 - smoothstep(twilight_w, 2.6 * twilight_w, abs(sun_height));

    let day_grad = mix(params.day_zenith_color.rgb, params.day_horizon_color.rgb, pow(horizon, params.gradient_params.x));
    let night_grad = mix(params.night_zenith_color.rgb, params.night_horizon_color.rgb, pow(horizon, params.gradient_params.y));
    return mix(night_grad, day_grad, day)
        + params.twilight_tint_color.rgb
        * pow(horizon, params.gradient_params.z)
        * twilight
        * params.gradient_params.w;
}

fn sky(ray: vec3<f32>, params: AtmosphereParams) -> vec3<f32> {
    let tau = 6.2831853;
    let pi = 3.14159265;

    let sun_dir = sky_sun_dir(params);
    let altitude = ray.y;
    let sun_height = sun_dir.y;
    let day = sky_day(sun_height, params);
    let night = 1.0 - day;
    var color = sky_gradient(ray, params);

    let sun_dot = dot(ray, sun_dir);
    let sun_visible = smoothstep(params.sun_params.x, params.sun_params.y, sun_height);
    let sun_disk = smoothstep(params.sun_params.z, params.sun_params.w, sun_dot) * sun_visible;
    let sun_corona = pow(max(sun_dot, 0.0), params.sun_star_params.x) * sun_visible;
    let sun_color = mix(params.sun_day_color.rgb, params.sun_night_color.rgb, day);
    color += sun_color * params.time_params.y * (2.1 * sun_disk + params.sun_star_params.y * sun_corona);

    // Stable sparse stars in lat-long cell space.
    let ray_n = normalize(ray);
    let lon = atan2(ray_n.z, ray_n.x);
    let lat = asin(clamp(ray_n.y, -1.0, 1.0));
    let uv = vec2<f32>(fract(lon / tau + 0.5), lat / pi + 0.5);
    let grid = vec2<f32>(320.0, 160.0);
    let cell = floor(uv * grid);
    let local = fract(uv * grid) - vec2<f32>(0.5);

    let seed = hash21(cell);
    let has_star = select(0.0, 1.0, seed > params.sun_star_params.z);
    let star_core = exp(-dot(local, local) * params.sun_star_params.w);
    let star = has_star
        * star_core
        * night
        * smoothstep(params.star_ground_params.x, params.star_ground_params.y, altitude);
    color += params.star_color.rgb * star * params.time_params.w;

    if altitude < 0.0 {
        let g = clamp(-altitude * params.star_ground_params.z, 0.0, 1.0);
        color = mix(color, mix(params.ground_night_color.rgb, params.ground_day_color.rgb, day), g);
    }

    return max(color, vec3<f32>(0.0));
}
