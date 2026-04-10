use wde_logger::prelude::*;

use bevy::{
    ecs::system::{
        SystemParamItem,
        lifetimeless::{SRes, SResMut}
    },
    prelude::*
};
use wde_renderer::prelude::*;

pub(crate) struct TerrainMaterialsPlugin;
impl Plugin for TerrainMaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainMaterialTextures>().add_plugins((
            ExtractResourcePlugin::<TerrainMaterialTextures>::default(),
            RenderBindingRegisterPlugin::<TerrainMaterialsBinding>::default(),
            RenderDataRegisterPlugin::<TerrainMaterials>::default()
        ));
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .add_systems(Render, fill_arrays.in_set(RenderSet::Prepare));
    }
}

const TEX_SIZE: (u32, u32) = (1024, 1024);
const MATERIALS: [&str; 4] = ["grass", "dirt", "rock", "sand"];
const TEX_TYPES: [&str; 4] = ["albedo", "normal", "roughness", "ambient_occlusion"];
const TEX_FORMATS: [TextureFormat; 4] = [
    TextureFormat::Rgba8UnormSrgb,
    TextureFormat::Rgba8Unorm,
    TextureFormat::R8Unorm,
    TextureFormat::R8Unorm
];

#[derive(Resource, Default)]
pub(crate) struct TerrainMaterialTextures {
    albedo_textures: Vec<Option<Handle<Texture>>>,
    normal_textures: Vec<Option<Handle<Texture>>>,
    roughness_textures: Vec<Option<Handle<Texture>>>,
    ao_textures: Vec<Option<Handle<Texture>>>
}
impl ExtractResource for TerrainMaterialTextures {
    type Source = Self;

    fn extract(source: &Self::Source) -> Self {
        Self {
            albedo_textures: source.albedo_textures.clone(),
            normal_textures: source.normal_textures.clone(),
            roughness_textures: source.roughness_textures.clone(),
            ao_textures: source.ao_textures.clone()
        }
    }
}

#[derive(Asset, Clone, Debug, TypePath, Default)]
pub(crate) struct TerrainMaterials;
impl TerrainMaterials {
    pub const TERRAIN_ALBEDO_IDX: u32 = 0;
    pub const TERRAIN_NORMAL_IDX: u32 = 1;
    pub const TERRAIN_ROUGHNESS_IDX: u32 = 2;
    pub const TERRAIN_AO_IDX: u32 = 3;
}
impl RenderData for TerrainMaterials {
    type Params = (SResMut<TerrainMaterialTextures>, SRes<AssetServer>);

    fn describe(
        (materials, asset_server): &mut SystemParamItem<Self::Params>,
        builder: &mut RenderDataBuilder
    ) {
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
        let usages = TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_DST
            | TextureUsages::RENDER_ATTACHMENT;
        builder
            .add_texture(
                Self::TERRAIN_ALBEDO_IDX,
                Texture {
                    label: "terrain_material_albedo_array".to_string(),
                    size,
                    format: TextureFormat::Rgba8UnormSrgb,
                    usages,
                    layer_count: materials.albedo_textures.len() as u32,
                    mip_level_count: 0, // Auto-calculate max mip levels
                    ..Default::default()
                }
            )
            .add_texture(
                Self::TERRAIN_NORMAL_IDX,
                Texture {
                    label: "terrain_material_normal_array".to_string(),
                    size,
                    format: TextureFormat::Rgba8Unorm,
                    usages,
                    layer_count: materials.normal_textures.len() as u32,
                    mip_level_count: 0, // Auto-calculate max mip levels
                    ..Default::default()
                }
            )
            .add_texture(
                Self::TERRAIN_ROUGHNESS_IDX,
                Texture {
                    label: "terrain_material_roughness_array".to_string(),
                    size,
                    format: TextureFormat::R8Unorm,
                    usages,
                    layer_count: materials.roughness_textures.len() as u32,
                    mip_level_count: 0, // Auto-calculate max mip levels
                    ..Default::default()
                }
            )
            .add_texture(
                Self::TERRAIN_AO_IDX,
                Texture {
                    label: "terrain_material_ao_array".to_string(),
                    size,
                    format: TextureFormat::R8Unorm,
                    usages,
                    layer_count: materials.ao_textures.len() as u32,
                    mip_level_count: 0, // Auto-calculate max mip levels
                    ..Default::default()
                }
            );
    }
}

#[derive(Asset, Clone, Debug, TypePath, Default)]
pub(crate) struct TerrainMaterialsBinding;
impl RenderBinding for TerrainMaterialsBinding {
    type Params = SRenderData<TerrainMaterials>;

    fn describe(
        &mut self,
        mat: &SystemParamItem<Self::Params>,
        builder: &mut RenderBindingBuilder
    ) {
        builder
            .add_texture_array_view(mat, TerrainMaterials::TERRAIN_ALBEDO_IDX)
            .add_texture_sampler(mat, TerrainMaterials::TERRAIN_ALBEDO_IDX)
            .add_texture_array_view(mat, TerrainMaterials::TERRAIN_NORMAL_IDX)
            .add_texture_sampler(mat, TerrainMaterials::TERRAIN_NORMAL_IDX)
            .add_texture_array_view(mat, TerrainMaterials::TERRAIN_ROUGHNESS_IDX)
            .add_texture_sampler(mat, TerrainMaterials::TERRAIN_ROUGHNESS_IDX)
            .add_texture_array_view(mat, TerrainMaterials::TERRAIN_AO_IDX)
            .add_texture_sampler(mat, TerrainMaterials::TERRAIN_AO_IDX);
    }

    fn label(&self) -> &str {
        "terrain_texture_arrays"
    }
}

/// Copy individual textures into the arrays and create the mipmaps
fn fill_arrays(
    render_instance: Res<RenderInstance>,
    textures: Res<RenderAssets<GpuTexture>>,
    materials: ResRenderData<TerrainMaterials>,
    material_textures: Res<TerrainMaterialTextures>,
    mut is_done: Local<bool>
) {
    // Only run once when all textures are loaded
    if *is_done {
        return;
    }

    // Get the array texture
    let materials = match materials.iter().next() {
        Some((_, materials)) => materials,
        None => return
    };
    let (albedo_array, normal_array, roughness_array, ao_array) = match (
        textures.get(
            &materials
                .get_texture(TerrainMaterials::TERRAIN_ALBEDO_IDX)
                .unwrap()
        ),
        textures.get(
            &materials
                .get_texture(TerrainMaterials::TERRAIN_NORMAL_IDX)
                .unwrap()
        ),
        textures.get(
            &materials
                .get_texture(TerrainMaterials::TERRAIN_ROUGHNESS_IDX)
                .unwrap()
        ),
        textures.get(
            &materials
                .get_texture(TerrainMaterials::TERRAIN_AO_IDX)
                .unwrap()
        )
    ) {
        (Some(albedo), Some(normal), Some(roughness), Some(ao)) => (albedo, normal, roughness, ao),
        _ => return
    };

    // Copy individual textures into the arrays
    let render_instance = render_instance.0.read().unwrap();
    let size = TEX_SIZE;
    for i in 0..material_textures.albedo_textures.len() {
        let albedo_tex = match textures.get(material_textures.albedo_textures[i].as_ref().unwrap())
        {
            Some(tex) => tex,
            None => return
        };
        albedo_array.texture.copy_from_texture_layered(
            &render_instance,
            &albedo_tex.texture.texture,
            i,
            size
        );

        let normal_tex = match textures.get(material_textures.normal_textures[i].as_ref().unwrap())
        {
            Some(tex) => tex,
            None => return
        };
        normal_array.texture.copy_from_texture_layered(
            &render_instance,
            &normal_tex.texture.texture,
            i,
            size
        );

        let roughness_tex =
            match textures.get(material_textures.roughness_textures[i].as_ref().unwrap()) {
                Some(tex) => tex,
                None => return
            };
        roughness_array.texture.copy_from_texture_layered(
            &render_instance,
            &roughness_tex.texture.texture,
            i,
            size
        );

        let ao_tex = match textures.get(material_textures.ao_textures[i].as_ref().unwrap()) {
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

    *is_done = true;
}
