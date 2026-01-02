use bevy::prelude::*;
use wde::prelude::*;

mod custom_material;
mod custom_pipeline;
mod custom_renderpass;
mod custom_ssbo;

pub use custom_material::*;
pub use custom_pipeline::*;
pub use custom_renderpass::*;
pub use custom_ssbo::*;


/// System to create the scene
fn create_scene(mut commands: Commands, asset_server: Res<AssetServer>, mut materials: ResMut<Assets<CustomMaterialAsset>>) {
    // Creates a camera
    commands.spawn((
        (
            Camera,
            Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y)
        ),
        CameraController::default()
    ));

    // Load the assets
    let box_texture = asset_server.load_with_settings("examples/custom_forward_render/box.png", 
    |settings: &mut TextureLoaderSettings| {
        settings.label = "custom-box".to_string();
        settings.format = TextureFormat::Rgba8Unorm;
        settings.usages = TextureUsages::TEXTURE_BINDING;
    });
    let red_box = materials.add(CustomMaterialAsset {
        label: "custom-material-red-box".to_string(),
        color: (1.0, 0.0, 0.0),
        texture: Some(box_texture),
    });
    let blue = materials.add(CustomMaterialAsset {
        label: "custom-material-blue".to_string(),
        color: (0.0, 0.0, 1.0),
        texture: None,
    });
    let suzanne = asset_server.load("examples/custom_forward_render/suzanne.obj");
    let cube = asset_server.load("examples/custom_forward_render/cube.obj");

    // Spawn the entities
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        Mesh(cube.clone()),
        CustomMaterial(blue.clone())
    ));
    commands.spawn((
        Transform::from_xyz(5.0, 0.0, 0.0),
        Mesh(cube.clone()),
        CustomMaterial(blue.clone())
    ));
    commands.spawn((
        Transform::from_xyz(10.0, 0.0, 0.0),
        Mesh(cube.clone()),
        CustomMaterial(red_box.clone())
    ));
    commands.spawn((
        Transform::from_xyz(15.0, 0.0, 0.0),
        Mesh(suzanne.clone()),
        CustomMaterial(red_box.clone())
    ));
}

/// Plugin to add the custom forward render pass, pipeline and material
pub struct CustomFeaturesPlugin;
impl Plugin for CustomFeaturesPlugin {
    fn build(&self, app: &mut App) {
        // Create the scene to display
        app
            .add_systems(Startup, create_scene);

        // Add the custom material
        app
            .add_plugins(MaterialsPluginRegister::<CustomMaterialAsset>::default());

        // Add the custom ssbo
        app
            .add_plugins(CustomSsboPlugin);

        // Add the custom pipeline
        app
            .init_asset::<CustomRenderPipelineAsset>()
            .add_plugins(RenderAssetsPlugin::<GpuCustomRenderPipeline>::default());

        // Add the custom render pass
        app.get_sub_app_mut(RenderApp).unwrap()
            .add_systems(Extract, CustomRenderPass::create_batches)
            .add_systems(Render, CustomRenderPass::render.in_set(RenderSet::Render));
    }

    fn finish(&self, app: &mut App) {
        // Create the render pass
        app.get_sub_app_mut(RenderApp).unwrap()
            .insert_resource(CustomRenderPass {
                batches: Vec::new()
            });

        // Create the pipeline
        let pipeline = app.world_mut()
            .get_resource::<AssetServer>().unwrap().add(CustomRenderPipelineAsset);
        app.get_sub_app_mut(RenderApp).unwrap().world_mut().spawn(CustomRenderPipeline(pipeline));
    }
}
