mod ghost_material;
mod ghost_pipeline;
mod ghost_subpass;

use bevy::prelude::*;

pub(crate) struct GhostPlugin;
impl Plugin for GhostPlugin {
    fn build(&self, _app: &mut App) {
        // // Register the material
        // app.add_plugins(MaterialsPluginRegister::<GhostMaterial>::default());

        // app.add_systems(Startup, init);
    }
}

// fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
//     let mat = GhostMaterial {
//         albedo: (0.0, 1.0, 0.0, 0.5)
//      };
//     commands.spawn((
//         Material3d(asset_server.add(mat)),
//         /* other components */
//     ));
// }
