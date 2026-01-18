//! Vertex structure and layout for a mesh.

/// Describe the vertex structure of a mesh.
/// 
/// # Fields
/// 
/// * `position` - The position of the vertex (location 0).
/// * `uv`       - The texture UV of the vertex (location 1).
/// * `normal`   - The normal of the vertex (location 2).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug, Default)]
pub struct Vertex {
    /// The position of the vertex.
    pub position: [f32; 3],
    /// The texture UV of the vertex (must be between 0.0 and 1.0).
    pub uv: [f32; 2],
    /// The normal of the vertex (must be normalized).
    pub normal: [f32; 3],
    /// The tangent of the vertex (xyz: tangent, w: handedness).
    pub tangent: [f32; 4],
}

impl Vertex {
    /// Describe the layout of the vertex.
    /// 
    /// # Returns
    /// 
    /// * `wgpu::VertexBufferLayout` - The layout of the vertex.
    ///
    /// # Example
    /// ```rust
    /// use wde_wgpu::vertex::Vertex;
    /// 
    /// let layout = Vertex::describe();
    /// assert_eq!(layout.array_stride, std::mem::size_of::<Vertex>() as u64);
    /// ```
    pub fn describe<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { // Position
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute { // UV
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute { // Normal
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute { // Tangent
                    offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
