// use bevy::prelude::*;
// use wde_renderer::prelude::*;

// #[repr(C)]
// #[derive(Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// struct GhostMaterialUniform {
//     pub albedo: [f32; 4]
// }

// #[derive(Asset, Clone, TypePath)]
// pub struct GhostMaterial {
//     pub albedo: (f32, f32, f32, f32)
// }
// impl Material for GhostMaterial {
//     fn describe(&self, builder: &mut MaterialBuilder) {
//         // Create the uniform buffer
//         let uniform = GhostMaterialUniform {
//             albedo: [self.albedo.0, self.albedo.1, self.albedo.2, self.albedo.3]
//         };

//         // Build the material
//         builder.add_buffer(
//             0, ShaderStages::FRAGMENT, BufferBindingType::Uniform,
//             size_of::<GhostMaterialUniform>(), Some(bytemuck::cast_slice(&[uniform]).to_vec()));
//     }

//     fn label(&self) -> &str { "ghost-material" }
// }
