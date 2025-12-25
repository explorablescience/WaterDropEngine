//! Instance of the GPU device required for the renderer.

use std::sync::{Arc, RwLock};

use bevy::{ecs::system::SystemState, log::{debug, error, warn, Level}, prelude::*, log::tracing::{event, span}, window::{PresentMode, PrimaryWindow, RawHandleWrapperHolder}};
use wgpu::{Device, Limits, Surface, SurfaceConfiguration, SurfaceTexture};

use crate::texture::WTextureView;

pub type WLimits = Limits;

/// Error type of the renderer.
#[derive(Debug)]
pub enum WRenderError {
    /// Cannot present render texture.
    CannotPresent,
    /// Cannot resize render instance.
    CannotResize,
    /// Pipeline not set.
    PipelineNotSet,
    /// Pipeline not initialized.
    PipelineNotInitialized,
    /// Missing a shader.
    MissingShader,
    /// Missing a vertex buffer.
    MissingVertexBuffer,
    /// Missing an index buffer.
    MissingIndexBuffer,
    /// Swapchain format not supported.
    UnsupportedSwapchainFormat,
    /// Depth format not supported.
    UnsupportedDepthFormat,
    /// Shader compilation error.
    ShaderCompilationError,
}

/// Type of the render texture.
#[derive(Debug)]
pub struct WRenderTexture {
    /// Texture of the render texture.
    pub texture: wgpu::SurfaceTexture,
    /// View of the render texture.
    pub view: WTextureView,
}

/// Type of the render event.
#[derive(Debug)]
pub enum WRenderEvent {
    /// Redraw the window.
    Redraw(WRenderTexture),
    /// Resize the window.
    Resize,
    /// No event.
    None,
}

/// Instance of the GPU device required for the renderer.
/// 
/// # Example
/// ```
/// let mut (...) = WRenderInstance::new("WaterDropEngine", &window).await;
/// WRenderInstance::setup_surface(...);
/// 
/// // Get current texture
/// let render_texture = WRenderInstance::get_current_texture(&instance);
/// 
/// // Render
/// // ...
/// 
/// // Present texture
/// WRenderInstance::present(render_texture);
/// 
/// // Resize the surface
/// // This must be called when the window is resized
/// WRenderInstance::resize(&instance.device, &instance.surface, &instance.surface_config);
/// ```
#[derive(Resource)]
pub struct WRenderInstance<'a> {
    pub data: Arc<RwLock<WRenderInstanceData<'a>>>,
}

/// Data of the render instance.
pub struct WRenderInstanceData<'a> {
    /// Device of the instance.
    pub device: Device,
    /// Queue of the instance.
    pub queue: wgpu::Queue,
    /// Surface of the instance.
    pub surface: Option<Surface<'a>>,
    /// Adapter of the instance.
    pub adapter: wgpu::Adapter,
    /// Instance of the GPU device.
    pub instance: wgpu::Instance,
    /// Surface configuration of the instance.
    pub surface_config: Option<SurfaceConfiguration>,
}

/// Create a new instance of the GPU device.
/// 
/// # Arguments
/// 
/// * `label` - Label of the instance.
/// * `app` - Application to create the instance.
pub async fn create_instance(label: &str, app: &mut App) -> WRenderInstance<'static> {
    info!(label, "Creating render instance.");
    let _trace = span!(Level::INFO, "new").entered();

    // Set flags
    let flags = if cfg!(debug_assertions) {
        wgpu::InstanceFlags::DEBUG | wgpu::InstanceFlags::VALIDATION
    } else {
        wgpu::InstanceFlags::DISCARD_HAL_LABELS
    };

    // Retrieve window
    let mut system_state: SystemState<Query<&RawHandleWrapperHolder, With<PrimaryWindow>>> = SystemState::new(app.world_mut());
    let primary_window = system_state.get(app.world()).single().ok().cloned();

    // Create wgpu instance
    debug!(label, "Creating wgpu instance.");
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        flags,
        dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
        gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
    });

    // Retrieve surface
    let surface = primary_window.and_then(|wrapper| unsafe {
        let maybe_handle = wrapper.0.lock().expect(
            "Couldn't get the window handle in time for renderer initialization",
        );
        if let Some(wrapper) = maybe_handle.as_ref() {
            let handle = wrapper.get_handle();
            Some(
                instance
                    .create_surface(handle)
                    .expect("Failed to create wgpu surface"),
            )
        } else { None }
    });

    // Retrieve adapter
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: surface.as_ref(),
            ..Default::default()
        })
        .await
        .unwrap_or_else(|| panic!("Failed to create adapter for '{}'.", label));

    // Check adaptater infos
    let adapter_info = adapter.get_info();
    info!("Using adapter named {} of {} type.", adapter_info.name, match adapter_info.device_type {
        wgpu::DeviceType::DiscreteGpu => "Discrete GPU",
        wgpu::DeviceType::IntegratedGpu => "Integrated GPU",
        wgpu::DeviceType::Cpu => "CPU",
        wgpu::DeviceType::VirtualGpu => "Virtual GPU",
        wgpu::DeviceType::Other => "Other",
    });
    if adapter_info.device_type == wgpu::DeviceType::Cpu {
        warn!("The selected adapter is using a driver that only supports software rendering, this will be very slow.");
    }

    // Set required features
    let required_features = wgpu::Features::INDIRECT_FIRST_INSTANCE
        | wgpu::Features::MULTI_DRAW_INDIRECT
        | wgpu::Features::PUSH_CONSTANTS;
        
    // Set limits
    let required_limits = Limits {
        max_push_constant_size: 128,
        max_compute_invocations_per_workgroup: 1024,
        ..Default::default()
    };

    // Create device instance and queue
    debug!(label, "Requesting device.");
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some(label), required_features, required_limits,
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        )
        .await
        .unwrap_or_else(|_| panic!("Failed to create device for '{}'.", label));

    // Log device infos
    debug!("Configured wgpu adapter Limits: {:#?}", device.limits());
    debug!("Configured wgpu adapter Features: {:#?}", device.features());

    // Return instance
    WRenderInstance {
        data: Arc::new(RwLock::new(WRenderInstanceData {
            device,
            queue,
            surface,
            adapter,
            instance,
            surface_config: None
        }))
    }
}

/// Setup the surface of the instance.
/// 
/// # Arguments
/// 
/// * `label` - Label of the instance.
/// * `size` - Size of the surface.
/// * `device` - Device of the instance.
/// * `surface` - Surface of the instance.
/// * `adapter` - Adapter of the instance.
/// * `present_mode` - Present mode of the instance.
/// 
/// # Returns
/// 
/// * `SurfaceConfiguration` - Surface configuration of the instance.
pub fn setup_surface(label: &str, size: (u32, u32), device: &Device, surface: &Surface, adapter: &wgpu::Adapter, present_mode: PresentMode) -> SurfaceConfiguration {
    debug!(label, "Configuring surface.");

    // Retrieve surface format (sRGB if possible)
    let surface_caps = surface.get_capabilities(adapter);
    let surface_format = surface_caps.formats.iter()
        .copied()
        .find(|f| f.is_srgb())
        .unwrap_or(surface_caps.formats[0]);

    // Set surface configuration
    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.0,
        height: size.1,
        present_mode: match present_mode {
            PresentMode::Fifo => wgpu::PresentMode::Fifo,
            PresentMode::FifoRelaxed => wgpu::PresentMode::FifoRelaxed,
            PresentMode::Mailbox => wgpu::PresentMode::Mailbox,
            PresentMode::Immediate => wgpu::PresentMode::Immediate,
            PresentMode::AutoVsync => wgpu::PresentMode::AutoVsync,
            PresentMode::AutoNoVsync => wgpu::PresentMode::AutoNoVsync,
        },
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2
    };
    surface.configure(device, &surface_config);

    surface_config
}

/// Get the render texture.
/// 
/// # Arguments
/// 
/// * `surface` - Surface of the instance.
/// * `surface_config` - Surface configuration of the instance.
/// 
/// # Returns
/// 
/// * `RenderEvent` - Render event.
pub fn get_current_texture(surface: &Surface, surface_config: &SurfaceConfiguration) -> WRenderEvent {
    event!(Level::TRACE, "Getting current texture.");

    // Get current texture
    let _get_current_texture = span!(Level::INFO, "acquire_texture").entered();
    let render_texture = surface.get_current_texture();
    drop(_get_current_texture);

    // Check if texture is acquired
    event!(Level::TRACE, "Texture acquired. Creating render view and checking status.");
    match render_texture {
        Ok(surface_texture) => {
            // Create render view
            let render_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("Main render texture"),
                format: match surface_config.format {
                    wgpu::TextureFormat::Bgra8UnormSrgb => Some(wgpu::TextureFormat::Bgra8UnormSrgb),
                    wgpu::TextureFormat::Rgba8UnormSrgb => Some(wgpu::TextureFormat::Rgba8UnormSrgb),
                    _ => panic!("Unsupported surface format.")
                },
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            });
            let cur_render = WRenderTexture {
                texture: surface_texture,
                view: render_view
            };
            WRenderEvent::Redraw(cur_render)
        }
        // Surface lost or outdated (minimized or moved to another screen)
        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
            WRenderEvent::Resize
        },
        // System out of memory
        Err(wgpu::SurfaceError::OutOfMemory) => {
            error!("System out of memory.");
            WRenderEvent::None
        }
        // Timeout of the surface
        Err(wgpu::SurfaceError::Timeout) => {
            error!("Timeout of the surface.");
            WRenderEvent::None
        }
    }
}

/// Present the render texture.
/// This must be called after the render function.
/// 
/// # Arguments
/// 
/// * `surface_texture` - Surface texture of the instance.
/// 
/// # Errors
/// 
/// * `RenderError::CannotPresent` - Cannot present render texture.
pub fn present(surface_texture: SurfaceTexture) -> Result<(), WRenderError> {
    event!(Level::TRACE, "Presenting render texture.");
    surface_texture.present();
    Ok(())
}

/// Resize the surface of the instance.
/// This must be called when the window is resized.
/// 
/// # Arguments
/// 
/// * `device` - Device of the instance.
/// * `surface` - Surface of the instance.
/// * `surface_config` - Surface configuration of the instance.
/// 
/// # Errors
/// 
/// * `RenderError::CannotResize` - Cannot resize render instance surface.
pub fn resize(device: &Device, surface: &Surface, surface_config: &SurfaceConfiguration) {
    event!(Level::DEBUG, "Resizing surface.");
    surface.configure(device, surface_config);
}
