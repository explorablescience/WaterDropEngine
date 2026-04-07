//! This module contains the definitions for a [`RenderBinding`](crate::assets::bindings::RenderBinding) and its corresponding [`GpuRenderBinding`](crate::assets::bindings::GpuRenderBinding), as well as utilities to define custom renderer bindings, such as custom materials.
//! It contains:
//! - A [`RenderBinding`](crate::assets::bindings::RenderBinding) is a representation of a binding group for a set of shader. It describe the resources that it provides (buffers, textures, samplers) and the shader stages that can access it. It is then extracted from the main world and prepared for the GPU as a [`GpuRenderBinding`](crate::assets::bindings::GpuRenderBinding), which contains the actual GPU resources (buffers, textures, samplers) and the bind group layout and bind group that can be used in the render pipeline.
//! - A [`Material`](crate::assets::bindings::Material) is a specific type of [`RenderBinding`](crate::assets::bindings::RenderBinding) that is used to define the resources and shader stages for a material. It is then extracted and prepared as a [`GpuMaterial`](crate::assets::bindings::GpuMaterial), which can be used in the render pipeline to render objects with that material.
//!
//! # Custom material example
//! Here below is an example of a custom material that provides a uniform buffer with the albedo color and an optional albedo texture.
//!
//! ## Material definition
//! The material is defined as a struct that implements the [`Material`](crate::assets::bindings::Material) and [`RenderBinding`](crate::assets::bindings::RenderBinding) traits. The `describe()` method of the [`RenderBinding`](crate::assets::bindings::RenderBinding) trait is used to define the resources and shader stages for the material. In this example, we create a uniform buffer for the albedo color and add optional texture and sampler bindings for the albedo texture.
//! ```
//! #[repr(C)]
//! #[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
//! struct PbrMaterialUniform {
//!     albedo: [f32; 4], // Albedo color of the material (r, g, b)
//! }
//!
//! #[derive(Asset, Clone, TypePath)]
//! struct PbrMaterial {
//!     label: String,
//!     albedo: (f32, f32, f32, f32),
//!     albedo_t: Option<Handle<Texture>>,
//! }
//! impl Material for PbrMaterial {}
//! impl RenderBinding for PbrMaterial {
//!     fn describe(&self, builder: &mut RenderBindingBuilder) {
//!        // Create the uniform buffer
//!        let uniform = PbrMaterialUniform {
//!            albedo: self.albedo,
//!        };
//!
//!        // Build the binding group
//!        builder.add_buffer(0, Buffer {
//!             label: format!("{}-uniform-buffer", self.label),
//!             size: std::mem::size_of::<PbrMaterialUniform>(),
//!             usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
//!             content: Some(bytemuck::cast_slice(&[uniform]).to_vec())
//!        });
//!        builder.add_texture_view   (1, ShaderStages::FRAGMENT, self.albedo_t.clone());
//!        builder.add_texture_sampler(2, ShaderStages::FRAGMENT, self.albedo_t.clone());
//!     }
//!
//!     fn label(&self) -> &str { &self.label }
//! }
//! ```
//! The expected shader code for this material would be as follows:
//! ```wgsl
//! struct PbrMaterialUniform {
//!    albedo: vec4<f32>, // Albedo color of the material (r, g, b, a)
//! };
//! @group(2) @binding(0) var<uniform> pbr_material: PbrMaterialUniform;    /// Material uniform buffer
//! @group(2) @binding(1) var albedo_texture: texture_2d<f32>;              /// Albedo texture (r, g, b)
//! @group(2) @binding(2) var albedo_sampler: sampler;                      /// Albedo texture sampler
//! ```
//!
//! The material should then be registered in the app with the [`MaterialsPluginRegister`](crate::assets::bindings::MaterialsPluginRegister) to register its extract commands and GPU preparation systems:
//! ```
//! app.add_plugins(MaterialsPluginRegister::<PbrMaterial>::default());
//! ```
//!
//! ## Material usage
//! An entity can then be rendered with this material by adding a [`Material3d`](crate::assets::bindings::Material3d) component with a handle to the material instance:
//! ```
//! let mat_handle = assets.add(PbrMaterial {
//!     label: "my_pbr_material".to_string(),
//!     albedo: (0.8, 0.7, 0.6, 1.0),
//!     albedo_t: Some(asset_server.load("res/my_albedo_texture.png")),
//! });
//! commands.spawn((
//!     /* (...) */
//!     Material3d(mat_handle),
//! ));
//! ```
//!
//! ## Updating material properties
//! The properties of the buffers and textures of the material can be updated by accessing the [`GpuMaterial`](crate::assets::bindings::GpuMaterial) in the render world and updating the corresponding GPU resources. You then get access to the raw wde_wgpu buffers and textures (see [`wde_wgpu::buffer::Buffer`](wde_wgpu::buffer::Buffer) and [`wde_wgpu::texture::Texture`](wde_wgpu::texture::Texture)).
//! For example, to update the albedo color of the material, you would do something like this (on the render app):
//! ```
//! // Get the resource
//! let gpu_materials: Binding<PbrMaterial> = /* ... */;
//! let material = match gpu_materials.iter().next() {
//!    Some((_, mat)) => mat,
//!    None => return
//! };
//!
//! // Get a buffer from the material
//! let gpu_buffers: RenderAssets<GpuBuffer> = /* ... */;
//! let buffer = match gpu_buffers.get(material.get_buffer(0).unwrap()) {
//!    Some(buffer) => buffer,
//!    None => return
//! };
//!
//! // Update the buffer content with new data
//! // (...)
//! ```
//!
//! An other possibility to update the properties is to recreate the instance with the new properties and replace the old one in the asset server:
//! ```
//! let new_mat_handle = assets.add(PbrMaterial {
//!    label: "my_pbr_material".to_string(),
//!    albedo: (0.9, 0.8, 0.7, 1.0),
//!    albedo_t: Some(asset_server.load("res/my_new_albedo_texture.png")),
//! });
//! commands.entity(my_entity).insert(Material3d(new_mat_handle));
//! ```
//!
//! ## Custom pipeline and subpass
//! See the documentation of [`crate::passes`](crate::passes) for more details on how to create custom render pipelines and subpasses that can use the material.
//!
//! # Generic render bindings
//! ## Main differences with materials
//! The [`RenderBinding`](crate::assets::bindings::RenderBinding) and [`GpuRenderBinding`](crate::assets::bindings::GpuRenderBinding) system can also be used for generic render bindings that are not materials. In this case, the workflow is essentially the same as for materials, with the main differences being:
//! - A generic render binding doesn't need to implement the [`Material`](crate::assets::bindings::Material) trait, only the [`RenderBinding`](crate::assets::bindings::RenderBinding) trait.
//! - The GPU representation of a generic render binding is a [`GpuRenderBinding`](crate::assets::bindings::GpuRenderBinding) instead of a [`GpuMaterial`](crate::assets::bindings::GpuMaterial) (but it's essentially an alias).
//! - The plugin to register the render binding is the [`RenderBindingPluginRegister`](crate::assets::bindings::RenderBindingPluginRegister) instead of the [`MaterialsPluginRegister`](crate::assets::bindings::MaterialsPluginRegister). Compared to the material plugin, the render binding plugin also creates a [`Resource`](bevy::prelude::Resource) containing a reference to the render binding.
//!   If you want to initialize yourself the render binding you can use:
//! ```
//! let render_binding_init_plugin = RenderBindingPluginRegister::<MyRenderBinding>::with_init(init, app);
//! app.add_plugins(render_binding_init_plugin);
//! ```
//! where `init` is a function with the following signature:
//! ```
//! fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     // Create the render binding
//!     let render_binding = MyRenderBinding { /* ... */ };
//!     let render_binding_handle = asset_server.add(render_binding);
//!     commands.insert_resource(RenderBindingHolder(render_binding_handle));
//! }
//! ```
//!
//! ## Updating generic render bindings
//! The properties of the buffers and textures of a generic render binding can be updated in the same way as for materials, by accessing the [`GpuRenderBinding`](crate::assets::bindings::GpuRenderBinding) in the render world and updating the corresponding GPU resources.
//! You can also recreate the instance with the new properties in the main world (using a function similar to the `init` function above) and replace the old [`RenderBindingHolder`](crate::assets::bindings::RenderBindingHolder) resource with the new one containing the new render binding instance.

use bevy::prelude::*;

use crate::{
    assets::bindings::{builder::RenderBindingsBuilderCache, dummy_texture::DummyTexturePlugin},
    core::RenderApp
};

mod builder;
mod dummy_texture;
mod material;
mod render_binding;

pub use builder::RenderBindingBuilder;
pub use material::*;
pub use render_binding::*;

pub(crate) struct MaterialsPlugin;
impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        // Add the dummy texture plugin (fallback texture for materials)
        app.add_plugins(DummyTexturePlugin);

        // Add cached resources
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<RenderBindingsBuilderCache>();
    }
}
