//! This module contains built-in meshes that can be used for rendering.
//! These include simple geometric shapes such as planes, cubes, and capsules. Each mesh is defined with its own vertex and index data, and can be easily instantiated and rendered in the scene.

mod plane;
mod cube;
mod capsule;

pub use plane::*;
pub use cube::*;
pub use capsule::*;
