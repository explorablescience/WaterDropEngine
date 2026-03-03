/// The size of each terrain tile in world units (e.g., 100.0 means each tile covers 100x100 units)
pub(crate) const TILE_SIZE: f32 = 100.0;
/// The number of subdivisions per tile (e.g., 16 means each tile is divided into a 16x16 grid of vertices)
pub(crate) const TILE_SUBDIVISIONS: u32 = 256;
/// The number of splat maps per tile (must be a multiple of 4, as each splat map can store 4 channels for texture blending)
pub(crate) const SPLAT_MAP_COUNT: u32 = 4;
