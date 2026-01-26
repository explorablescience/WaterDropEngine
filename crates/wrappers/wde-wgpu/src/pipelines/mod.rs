//! Pipeline and bind-group construction utilities.
//!
//! - [`render_pipeline`]: WGSL vertex/fragment pipelines with depth, topology, and push constants.
//! - [`compute_pipeline`]: WGSL compute pipelines with bind groups and push constants.
//! - [`bind_group`]: builders for layouts and concrete bind groups aligned to WGSL bindings.
pub mod compute_pipeline;
pub mod render_pipeline;
pub mod bind_group;

pub use bind_group::*;
pub use compute_pipeline::*;
pub use render_pipeline::*;
