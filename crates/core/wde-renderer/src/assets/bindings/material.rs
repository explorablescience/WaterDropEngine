use crate::prelude::*;
use bevy::{ecs::system::lifetimeless::SRes, prelude::*};

/// Alias for a `SRes<RenderAssets<GpuMaterial<M>>>`.
pub type SMaterial<M> = SRes<RenderAssets<GpuMaterial<M>>>;

/// Utils component that stores a [`Material`] asset handle for 3D rendering.
#[derive(Component, Clone)]
pub struct Material3d<M: RenderBinding>(pub Handle<M>);

/// A material is a render asset that describes GPU resources (buffers and textures) to be used for rendering, and their binding metadata (binding index, visibility, etc). It is created by implementing the [`RenderBinding`] trait for a custom asset type, and describing the material's resources in the `describe` method using the provided [`RenderBindingBuilder`].
/// It is an alias for a [`RenderBinding`].
pub trait Material: RenderBinding + Sync + Send + Asset + Clone {}
/// A GPU material is a render asset created from a material, containing the GPU resources described by the material and used for rendering.
/// It is an alias for a [`GpuRenderBinding`] of the material type.
pub type GpuMaterial<M> = GpuRenderBinding<M>;

/// Plugin to register a custom [`Material`] type as an asset, and prepare the corresponding [`GpuMaterial`] render asset.
pub struct MaterialsPluginRegister<M: Material> {
    _phantom: std::marker::PhantomData<M>
}
impl<M: Material> Default for MaterialsPluginRegister<M> {
    fn default() -> Self {
        MaterialsPluginRegister {
            _phantom: std::marker::PhantomData
        }
    }
}
impl<M: Material> Plugin for MaterialsPluginRegister<M> {
    fn build(&self, app: &mut App) {
        app.init_asset::<M>()
            .add_plugins(RenderAssetsPlugin::<GpuMaterial<M>>::default());
    }
}
