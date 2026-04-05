use bevy::prelude::*;
use wde_logger::prelude::*;
use wde_renderer::prelude::*;

const TEX_SIZE: (u32, u32) = (1024, 1024);
const MATERIALS: [&str; 4] = ["grass", "dirt", "rock", "sand"];
const TEX_TYPES: [&str; 4] = ["albedo", "normal", "roughness", "ambient_occlusion"];
const TEX_FORMATS: [TextureFormat; 4] = [
    TextureFormat::Rgba8UnormSrgb,
    TextureFormat::Rgba8Unorm,
    TextureFormat::R8Unorm,
    TextureFormat::R8Unorm
];

/// Resource storing individual texture handles for each material
#[derive(Resource, Default)]
pub struct TerrainMaterials {
    // Individual texture handles for each material and type
    pub albedo_textures: Vec<Option<Handle<Texture>>>,
    pub normal_textures: Vec<Option<Handle<Texture>>>,
    pub roughness_textures: Vec<Option<Handle<Texture>>>,
    pub ao_textures: Vec<Option<Handle<Texture>>>,

    // Array textures
    pub albedo_array: Option<Handle<Texture>>,
    pub normal_array: Option<Handle<Texture>>,
    pub roughness_array: Option<Handle<Texture>>,
    pub ao_array: Option<Handle<Texture>>
}

/// Resource storing the GPU texture arrays and bind group
#[derive(Resource, Default)]
pub struct TerrainMaterialArrays {
    pub bind_group_layout: Option<BindGroupLayout>,
    pub bind_group: Option<BindGroup>
}

pub struct TerrainMaterialsPlugin;
impl Plugin for TerrainMaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainMaterials>()
            .add_systems(Startup, load_material_textures);

        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<TerrainMaterials>()
            .init_resource::<TerrainMaterialArrays>()
            .add_systems(Extract, extract_terrain_textures)
            .add_systems(Render, build_material_arrays.in_set(RenderSet::BindGroups));
    }
}

/// Load individual material textures
fn load_material_textures(asset_server: Res<AssetServer>, mut materials: ResMut<TerrainMaterials>) {
    // Load textures for each material and type
    let usages = TextureUsages::COPY_SRC | TextureUsages::TEXTURE_BINDING;
    for material in MATERIALS {
        for (i, tex_type) in TEX_TYPES.iter().enumerate() {
            let path = format!("core/models/terrain/{}/{}.png", material, tex_type);
            let handle = asset_server.load_with_settings(
                path,
                move |settings: &mut TextureLoaderSettings| {
                    settings.label = format!("terrain-{}-{}", material, tex_type);
                    settings.format = TEX_FORMATS[i];
                    settings.usages = usages;
                }
            );
            match i {
                0 => materials.albedo_textures.push(Some(handle)),
                1 => materials.normal_textures.push(Some(handle)),
                2 => materials.roughness_textures.push(Some(handle)),
                3 => materials.ao_textures.push(Some(handle)),
                _ => unreachable!()
            }
        }
    }

    // Build texture arrays for each type
    let size = TEX_SIZE; // Assume all textures have the same size
    let usages =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let albedo_array = asset_server.add(Texture {
        label: "terrain_material_albedo_array".to_string(),
        size,
        format: TextureFormat::Rgba8UnormSrgb,
        usages,
        layer_count: materials.albedo_textures.len() as u32,
        mip_level_count: 0, // Auto-calculate max mip levels
        ..Default::default()
    });
    let normal_array = asset_server.add(Texture {
        label: "terrain_material_normal_array".to_string(),
        size,
        format: TextureFormat::Rgba8Unorm,
        usages,
        layer_count: materials.normal_textures.len() as u32,
        mip_level_count: 0, // Auto-calculate max mip levels
        ..Default::default()
    });
    let roughness_array = asset_server.add(Texture {
        label: "terrain_material_roughness_array".to_string(),
        size,
        format: TextureFormat::R8Unorm,
        usages,
        layer_count: materials.roughness_textures.len() as u32,
        mip_level_count: 0, // Auto-calculate max mip levels
        ..Default::default()
    });
    let ao_array = asset_server.add(Texture {
        label: "terrain_material_ao_array".to_string(),
        size,
        format: TextureFormat::R8Unorm,
        usages,
        layer_count: materials.ao_textures.len() as u32,
        mip_level_count: 0, // Auto-calculate max mip levels
        ..Default::default()
    });
    materials.albedo_array = Some(albedo_array);
    materials.normal_array = Some(normal_array);
    materials.roughness_array = Some(roughness_array);
    materials.ao_array = Some(ao_array);
}

/// Extract terrain tile textures and build bind groups for the terrain pass
fn extract_terrain_textures(
    terrain_materials_world: ExtractWorld<Res<TerrainMaterials>>,
    mut terrain_materials: ResMut<TerrainMaterials>,
    mut terrain_materials_bg: ResMut<TerrainMaterialArrays>
) {
    // Check if bind group is already built or if all textures are loaded
    if terrain_materials_bg.bind_group.is_some() {
        return;
    }
    // Check if all textures are loaded
    if terrain_materials_world.albedo_textures.is_empty()
        || terrain_materials_world.albedo_array.is_none()
    {
        return;
    }

    // Just clone the handles, the actual GPU texture arrays and bind groups will be built in the render world
    terrain_materials.albedo_textures = {
        let mut textures = Vec::new();
        for i in 0..terrain_materials_world.albedo_textures.len() {
            let tex = terrain_materials_world.albedo_textures[i].clone();
            textures.push(tex);
        }
        textures
    };
    terrain_materials.normal_textures = {
        let mut textures = Vec::new();
        for i in 0..terrain_materials_world.normal_textures.len() {
            let tex = terrain_materials_world.normal_textures[i].clone();
            textures.push(tex);
        }
        textures
    };
    terrain_materials.roughness_textures = {
        let mut textures = Vec::new();
        for i in 0..terrain_materials_world.roughness_textures.len() {
            let tex = terrain_materials_world.roughness_textures[i].clone();
            textures.push(tex);
        }
        textures
    };
    terrain_materials.ao_textures = {
        let mut textures = Vec::new();
        for i in 0..terrain_materials_world.ao_textures.len() {
            let tex = terrain_materials_world.ao_textures[i].clone();
            textures.push(tex);
        }
        textures
    };

    // Also clone the array textures
    terrain_materials.albedo_array = terrain_materials_world.albedo_array.clone();
    terrain_materials.normal_array = terrain_materials_world.normal_array.clone();
    terrain_materials.roughness_array = terrain_materials_world.roughness_array.clone();
    terrain_materials.ao_array = terrain_materials_world.ao_array.clone();

    // Extract the material arrays bind group
    terrain_materials_bg.bind_group_layout = terrain_materials_bg.bind_group_layout.clone();
    terrain_materials_bg.bind_group = terrain_materials_bg.bind_group.clone();
}

/// Copy individual textures into the arrays and build the bind group for the material arrays
fn build_material_arrays(
    render_instance: Res<RenderInstance>,
    textures: Res<RenderAssets<GpuTexture>>,
    materials: Res<TerrainMaterials>,
    mut material_arrays: ResMut<TerrainMaterialArrays>
) {
    // Check if bind group is already built or if all textures are loaded
    if material_arrays.bind_group.is_some()
        || (materials.albedo_textures.iter().any(|tex| tex.is_none())
            || materials.albedo_array.is_none())
    {
        return;
    }

    // Get the array texture
    let (albedo_array, normal_array, roughness_array, ao_array) = match (
        (materials
            .albedo_array
            .as_ref()
            .and_then(|handle| textures.get(handle))),
        (materials
            .normal_array
            .as_ref()
            .and_then(|handle| textures.get(handle))),
        (materials
            .roughness_array
            .as_ref()
            .and_then(|handle| textures.get(handle))),
        (materials
            .ao_array
            .as_ref()
            .and_then(|handle| textures.get(handle)))
    ) {
        (Some(albedo), Some(normal), Some(roughness), Some(ao)) => (albedo, normal, roughness, ao),
        _ => return
    };

    // Copy individual textures into the arrays
    let render_instance = render_instance.0.read().unwrap();
    let size = TEX_SIZE;
    for i in 0..materials.albedo_textures.len() {
        let albedo_tex = match textures.get(materials.albedo_textures[i].as_ref().unwrap()) {
            Some(tex) => tex,
            None => return
        };
        albedo_array.texture.copy_from_texture_layered(
            &render_instance,
            &albedo_tex.texture.texture,
            i,
            size
        );

        let normal_tex = match textures.get(materials.normal_textures[i].as_ref().unwrap()) {
            Some(tex) => tex,
            None => return
        };
        normal_array.texture.copy_from_texture_layered(
            &render_instance,
            &normal_tex.texture.texture,
            i,
            size
        );

        let roughness_tex = match textures.get(materials.roughness_textures[i].as_ref().unwrap()) {
            Some(tex) => tex,
            None => return
        };
        roughness_array.texture.copy_from_texture_layered(
            &render_instance,
            &roughness_tex.texture.texture,
            i,
            size
        );

        let ao_tex = match textures.get(materials.ao_textures[i].as_ref().unwrap()) {
            Some(tex) => tex,
            None => return
        };
        ao_array.texture.copy_from_texture_layered(
            &render_instance,
            &ao_tex.texture.texture,
            i,
            size
        );
    }

    // Generate mipmaps for all texture arrays
    debug!("Generating mipmaps for terrain material arrays.");
    albedo_array.texture.generate_mipmaps(&render_instance);
    normal_array.texture.generate_mipmaps(&render_instance);
    roughness_array.texture.generate_mipmaps(&render_instance);
    ao_array.texture.generate_mipmaps(&render_instance);

    // Build the bind group for the material arrays
    let bind_group_layout = BindGroupLayout::new(
        "terrain_material_arrays",
        |builder: &mut BindGroupLayoutBuilder| {
            builder.add_texture_array_view(0, ShaderStages::FRAGMENT);
            builder.add_texture_sampler(1, ShaderStages::FRAGMENT);
            builder.add_texture_array_view(2, ShaderStages::FRAGMENT);
            builder.add_texture_sampler(3, ShaderStages::FRAGMENT);
            builder.add_texture_array_view(4, ShaderStages::FRAGMENT);
            builder.add_texture_sampler(5, ShaderStages::FRAGMENT);
            builder.add_texture_array_view(6, ShaderStages::FRAGMENT);
            builder.add_texture_sampler(7, ShaderStages::FRAGMENT);
        }
    );

    // Create the bind group
    let bind_group = BindGroupBuilder::build(
        "terrain_material_arrays",
        &render_instance,
        &BindGroupLayout::build(&bind_group_layout, &render_instance).unwrap(),
        &vec![
            BindGroupBuilder::texture_view(0, &albedo_array.texture),
            BindGroupBuilder::texture_sampler(1, &albedo_array.texture),
            BindGroupBuilder::texture_view(2, &normal_array.texture),
            BindGroupBuilder::texture_sampler(3, &normal_array.texture),
            BindGroupBuilder::texture_view(4, &roughness_array.texture),
            BindGroupBuilder::texture_sampler(5, &roughness_array.texture),
            BindGroupBuilder::texture_view(6, &ao_array.texture),
            BindGroupBuilder::texture_sampler(7, &ao_array.texture),
        ]
    )
    .unwrap();

    material_arrays.bind_group = Some(bind_group);
    material_arrays.bind_group_layout = Some(bind_group_layout);
}
