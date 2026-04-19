//! Texture creation and copy helpers built on `wgpu::Texture`.

use wde_logger::prelude::*;

use crate::RenderInstanceData;

/// Surface texture.
pub type SurfaceTexture = wgpu::SurfaceTexture;

/// Texture view
pub type TextureView = wgpu::TextureView;

/// Texture usages.
pub type TextureUsages = wgpu::TextureUsages;

/// Texture format.
pub type TextureFormat = wgpu::TextureFormat;

/// Texture filter mode.
pub type FilterMode = wgpu::FilterMode;

/// The swapchain texture format.
pub const SWAPCHAIN_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;
/// The depth texture format.
pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

/// Texture wrapper with a ready-to-use view and sampler.
///
/// # Examples
/// Create a render target and clear it:
/// ```rust,no_run
/// use wde_wgpu::{texture::{Texture, TextureFormat, TextureUsages}, instance::RenderInstanceData};
///
/// let color = Texture::new(
///     instance,
///     "color-target",
///     (1280, 720),
///     TextureFormat::Rgba8Unorm,
///     TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
/// );
/// ```
///
/// Upload raw pixel data (RGBA8):
/// ```rust,no_run
/// use wde_wgpu::{texture::{Texture, TextureFormat, TextureUsages}, instance::RenderInstanceData};
///
/// let texture = Texture::new(
///     instance,
///     "albedo",
///     (512, 512),
///     TextureFormat::Rgba8Unorm,
///     TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
/// );
/// texture.copy_from_buffer(instance, TextureFormat::Rgba8Unorm, pixels);
/// // Or
/// texture.copy_from_buffer_layered(instance, TextureFormat::Rgba8Unorm, 0, pixels); // For texture arrays
/// ```
///
/// Copy one GPU texture into another:
/// ```rust,no_run
/// # use wde_wgpu::{texture::{Texture, TextureFormat, TextureUsages}, instance::RenderInstanceData};
///
/// let dst = Texture::new(
///     instance,
///     "blit-target",
///     (1024, 1024),
///     TextureFormat::Rgba8Unorm,
///     TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
/// );
/// dst.copy_from_texture(instance, src, (1024, 1024));
/// ```
pub struct Texture {
    pub label: String,
    pub texture: wgpu::Texture,
    pub format: TextureFormat,
    pub view: TextureView,
    pub sampler: wgpu::Sampler,
    pub size: (u32, u32),
    pub sample_count: u32,
    pub layer_count: u32,
    pub mip_level_count: u32
}

impl std::fmt::Debug for Texture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texture")
            .field("label", &self.label)
            .field("sampler", &self.sampler)
            .field("size", &self.size)
            .field("sample_count", &self.sample_count)
            .field("layer_count", &self.layer_count)
            .field("mip_level_count", &self.mip_level_count)
            .finish()
    }
}

impl Texture {
    /// Create a new texture.
    ///
    /// # Arguments
    ///
    /// * `instance` - Game instance.
    /// * `label` - Label of the texture. This is only for debugging purposes.
    /// * `size` - Size of the texture (width, height).
    /// * `format` - Format of the texture (e.g. Rgba8Unorm, Depth32Float, etc.).
    /// * `usage` - Usage of the texture (e.g. RENDER_ATTACHMENT, COPY_SRC, COPY_DST, etc.).
    /// * `sample_count` - Sample count of the texture (e.g. 1 for no MSAA, 4 for 4x MSAA, etc.).
    /// * `layer_count` - Number of layers in the texture array. Default is 1 (for non-array textures).
    /// * `mip_level_count` - Number of mip levels. Default is 1 (no mipmaps). If set to 0, it will be auto-calculated based on the texture size.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instance: &RenderInstanceData<'_>,
        label: &str,
        size: (u32, u32),
        format: TextureFormat,
        usage: TextureUsages,
        sample_count: u32,
        layer_count: u32,
        mip_level_count: u32
    ) -> Self {
        event!(LogLevel::TRACE, "Creating wgpu texture {}.", label);

        // Calculate max mip levels if requested
        let mip_level_count = if mip_level_count == 0 {
            (size.0.max(size.1) as f32).log2().floor() as u32 + 1
        } else {
            mip_level_count
        };

        // Create texture
        let texture = instance.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(format!("{}-texture", label).as_str()),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: layer_count
            },
            mip_level_count,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: usage | wgpu::TextureUsages::COPY_DST,
            view_formats: &[]
        });

        // Create texture view
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(format!("{}-texture-view", label).as_str()),
            format: if format == DEPTH_FORMAT {
                None
            } else {
                Some(format)
            },
            dimension: if format == DEPTH_FORMAT {
                None
            } else if layer_count > 1 {
                Some(wgpu::TextureViewDimension::D2Array)
            } else {
                Some(wgpu::TextureViewDimension::D2)
            },
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            base_array_layer: 0,
            mip_level_count: None,
            array_layer_count: if layer_count > 1 {
                Some(layer_count)
            } else {
                None
            },
            usage: None
        });

        // Create sampler
        let sampler = instance.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(format!("{}-texture-sampler", label).as_str()),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: if mip_level_count > 1 {
                wgpu::FilterMode::Linear
            } else {
                wgpu::FilterMode::Nearest
            },
            lod_min_clamp: 0.0,
            lod_max_clamp: mip_level_count as f32,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None
        });

        // Return texture
        Self {
            label: label.to_string(),
            texture,
            format,
            view,
            sampler,
            size,
            sample_count,
            layer_count,
            mip_level_count
        }
    }

    /// Copy buffer to texture.
    /// It is assumed that the buffer is the same size as the texture.
    /// It will be copied on the next queue submit.
    /// Note that the buffer must have the COPY_DST usage.
    ///
    /// # Arguments
    ///
    /// * `instance` - Game instance.
    /// * `texture_format` - The wgpu texture format.
    /// * `buffer` - Image buffer.
    pub fn copy_from_buffer(
        &self,
        instance: &RenderInstanceData,
        texture_format: TextureFormat,
        buffer: &[u8]
    ) {
        // Retrieve size corresponding to the texture format
        let format_size = match texture_format.block_dimensions() {
            (1, 1) => texture_format.block_copy_size(None).unwrap() as usize,
            _ => panic!("Using pixel_size for compressed textures is invalid")
        };

        // Copy buffer to texture
        instance.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All
            },
            buffer,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.size.0 * format_size as u32),
                rows_per_image: None
            },
            wgpu::Extent3d {
                width: self.size.0,
                height: self.size.1,
                depth_or_array_layers: 1
            }
        );
    }

    /// Copy texture to buffer at a given array layer.
    /// It is assumed that the buffer is the same size as the texture.
    /// It will be copied on the next queue submit.
    /// Note that the buffer must have the COPY_DST usage.
    ///
    /// # Arguments
    ///
    /// * `instance` - Game instance.
    /// * `texture_format` - The wgpu texture format.
    /// * `array_layer` - The array layer to copy from (for texture arrays). Default is 0 for non-array textures.
    /// * `buffer` - Image buffer.
    pub fn copy_from_buffer_layered(
        &self,
        instance: &RenderInstanceData,
        texture_format: TextureFormat,
        array_layer: u32,
        buffer: &[u8]
    ) {
        // Retrieve size corresponding to the texture format
        let format_size = match texture_format.block_dimensions() {
            (1, 1) => texture_format.block_copy_size(None).unwrap() as usize,
            _ => panic!("Using pixel_size for compressed textures is invalid")
        };

        // Copy buffer to texture
        instance.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: array_layer
                },
                aspect: wgpu::TextureAspect::All
            },
            buffer,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.size.0 * format_size as u32),
                rows_per_image: None
            },
            wgpu::Extent3d {
                width: self.size.0,
                height: self.size.1,
                depth_or_array_layers: 1
            }
        );
    }

    /// Copy texture to texture.
    /// It is assumed that the texture is the same size as the source texture.
    /// Note that the input texture must have the COPY_SRC usage, and the output texture must have the COPY_DST usage.
    ///
    /// # Arguments
    ///
    /// * `instance` - Game instance.
    /// * `texture` - Texture to copy from.
    /// * `size` - Size of the texture.
    pub fn copy_from_texture(
        &self,
        instance: &RenderInstanceData<'_>,
        texture: &wgpu::Texture,
        size: (u32, u32)
    ) {
        // Create command buffer
        let mut command = crate::command_buffer::CommandBuffer::new(instance, "Copy Texture");

        // Copy texture to texture
        command.encoder().copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All
            },
            wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1
            }
        );

        // Submit the commands
        command.submit(instance);
    }

    /// Copy texture from a surface texture (e.g. swapchain frame).
    /// It is assumed that the texture is the same size as the source texture.
    /// Note that the input texture must have the COPY_SRC usage, and the output texture must have the COPY_DST usage.
    ///
    /// # Arguments
    ///
    /// * `instance` - Game instance.
    /// * `surface_texture` - Surface texture to copy from.
    /// * `size` - Size of the texture.
    pub fn copy_from_surface_texture(
        &self,
        instance: &RenderInstanceData<'_>,
        surface_texture: &SurfaceTexture,
        size: (u32, u32)
    ) {
        self.copy_from_texture(instance, &surface_texture.texture, size);
    }

    /// Copy texture to texture at a given array layer.
    /// It is assumed that the texture is the same size as the source texture.
    /// Note that the input texture must have the COPY_SRC usage, and the output texture must have the COPY_DST usage.
    ///
    /// # Arguments
    ///
    /// * `instance` - Game instance.
    /// * `texture` - Texture to copy from.
    /// * `array_layer` - The array layer to copy to (for texture arrays). Default is 0 for non-array textures.
    /// * `size` - Size of the texture.
    pub fn copy_from_texture_layered(
        &self,
        instance: &RenderInstanceData<'_>,
        texture: &wgpu::Texture,
        array_layer: usize,
        size: (u32, u32)
    ) {
        // Create command buffer
        let mut command = crate::command_buffer::CommandBuffer::new(instance, "Copy Texture");

        // Copy texture to texture
        command.encoder().copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: array_layer as u32
                },
                aspect: wgpu::TextureAspect::All
            },
            wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1
            }
        );

        // Submit the commands
        command.submit(instance);
    }

    /// Generate mipmaps for this texture.
    /// The texture must have been created with TEXTURE_BINDING | RENDER_ATTACHMENT | COPY_SRC usage.
    /// This method uses a simple blit approach to downsample each mip level from the previous one.
    ///
    /// # Arguments
    ///
    /// * `instance` - Game instance.
    pub fn generate_mipmaps(&self, instance: &RenderInstanceData<'_>) {
        if self.mip_level_count <= 1 {
            trace!(
                "Texture {} does not have multiple mip levels ({}), skipping mipmap generation.",
                self.label, self.mip_level_count
            );
            return;
        }

        trace!(
            "Generating mipmaps for texture {} with {} mip levels and {} layers.",
            self.label, self.mip_level_count, self.layer_count
        );

        // Create the blit shader module
        let shader_source = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32((vertex_index << 1u) & 2u);
    let y = f32(vertex_index & 2u);
    out.position = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    out.tex_coord = vec2<f32>(x, 1.0 - y);
    return out;
}

@group(0) @binding(0) var src_texture: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(src_texture, src_sampler, in.tex_coord);
}
"#;

        let shader = instance
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("mipmap_blit_shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into())
            });

        // Create sampler for mipmap generation (linear filtering for downsampling)
        let blit_sampler = instance.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mipmap_blit_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create bind group layout
        let bind_group_layout =
            instance
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("mipmap_blit_bind_group_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false
                            },
                            count: None
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None
                        }
                    ]
                });

        // Create pipeline layout
        let pipeline_layout =
            instance
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("mipmap_blit_pipeline_layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[]
                });

        // Create render pipeline
        let pipeline = instance
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("mipmap_blit_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL
                    })],
                    compilation_options: Default::default()
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None
            });

        // Generate each mip level
        let mut command = crate::command_buffer::CommandBuffer::new(instance, "Generate Mipmaps");

        for layer in 0..self.layer_count {
            for mip_level in 1..self.mip_level_count {
                let src_view = self.texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some(&format!("{}_mip_{}_src", self.label, mip_level)),
                    format: Some(self.format),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: mip_level - 1,
                    mip_level_count: Some(1),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                    usage: None
                });

                let dst_view = self.texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some(&format!("{}_mip_{}_dst", self.label, mip_level)),
                    format: Some(self.format),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: mip_level,
                    mip_level_count: Some(1),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                    usage: None
                });

                let bind_group = instance
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some(&format!("{}_mip_{}_bind_group", self.label, mip_level)),
                        layout: &bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&src_view)
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&blit_sampler)
                            }
                        ]
                    });

                {
                    let mut render_pass =
                        command
                            .encoder()
                            .begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some(&format!(
                                    "{}_mip_{}_render_pass",
                                    self.label, mip_level
                                )),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &dst_view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                        store: wgpu::StoreOp::Store
                                    },
                                    depth_slice: None
                                })],
                                depth_stencil_attachment: None,
                                timestamp_writes: None,
                                occlusion_query_set: None
                            });

                    render_pass.set_pipeline(&pipeline);
                    render_pass.set_bind_group(0, &bind_group, &[]);
                    render_pass.draw(0..3, 0..1);
                }
            }
        }

        command.submit(instance);
        event!(
            LogLevel::TRACE,
            "Finished generating mipmaps for texture {}.",
            self.label
        );
    }
}
