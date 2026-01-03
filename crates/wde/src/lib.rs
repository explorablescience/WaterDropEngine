// Prelude
pub mod prelude {
    pub use wde_renderer::prelude::*;
    pub use wde_camera::prelude::*;
    pub use wde_physics::prelude::*;
    // pub use wde_scene::prelude::*;

    #[cfg(feature = "gizmos")]
    pub use wde_gizmos::prelude::*;

    #[cfg(feature = "pbr")]
    pub use wde_pbr::prelude::*;
}

// Reexport modules
pub mod render {
    pub use wde_renderer::*;
    pub mod camera {
        pub use wde_camera::*;
    }
}
pub mod scene {
    pub use wde_scene::*;
}

#[cfg(feature = "gizmos")]
pub mod gizmos {
    pub use wde_gizmos::*;
}

#[cfg(feature = "pbr")]
pub mod pbr {
    pub use wde_pbr::*;
}

