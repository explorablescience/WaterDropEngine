//! Device/queue/surface bootstrap utilities used across the renderer.

use bevy::{tasks::block_on, window::RawHandleWrapperHolder};
use wde_logger::prelude::*;
use wgpu::{BackendOptions, Device, Features, Limits as WLimits, MemoryBudgetThresholds, PresentMode as WPresentMode, Surface, SurfaceConfiguration, SurfaceTexture};

use crate::texture::TextureView;

pub type Limits = WLimits;
pub type PresentMode = WPresentMode;

/// Errors emitted by the renderer wrappers.
///
/// Match on this when guarding pipeline creation or pass submission:
/// ```rust,no_run
/// use wde_wgpu::instance::RenderError;
/// 
/// fn handle(error: RenderError) {
///     match error {
///         RenderError::MissingShader => eprintln!("shader missing"),
///         RenderError::PipelineNotInitialized => eprintln!("call init() before drawing"),
///         _ => eprintln!("other GPU error: {:?}", error),
///     }
/// }
/// ```
#[derive(Debug)]
pub enum RenderError {
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
    /// Missing a bind group required by the shader.
    MissingBindGroup,
}

/// Swapchain image plus an already-created view.
#[derive(Debug)]
pub struct RenderTexture {
    /// Texture of the render texture.
    pub texture: wgpu::SurfaceTexture,
    /// View of the render texture.
    pub view: TextureView,
}

/// Results of attempting to acquire the next surface texture.
///
/// The [`RenderEvent::Resize`] variant signals that the swapchain must be reconfigured
/// with [`resize`].
#[derive(Debug)]
pub enum RenderEvent {
    /// Redraw the window.
    Redraw(RenderTexture),
    /// Resize the window.
    Resize,
    /// No event.
    None,
}

/// Device, queue, surface, and adapter bundle used to create GPU resources.
///
/// This is the handle you pass into buffers, textures, command buffers, and pipelines.
/// The type is intentionally lightweight to clone/read behind the `Arc<RwLock<_>>` in
/// `RenderInstance`.
///
/// ```rust,no_run
/// use wde_wgpu::{buffer::{Buffer, BufferUsage}, instance::{create_instance, RenderInstanceData}};
/// 
/// let instance = create_instance("docs", Some(&window)).await;
/// let data: std::sync::RwLockReadGuard<'_, RenderInstanceData> = instance.data.read().unwrap();
/// 
/// let mut buf = Buffer::new(&data, "hello", 16, BufferUsage::UNIFORM | BufferUsage::COPY_DST, None);
/// buf.write(&data, &[1, 2, 3, 4], 0);
/// ```
pub struct RenderInstanceData<'a> {
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

/// Create a new GPU instance (device + queue) and optionally bind to a window surface.
///
/// `primary_window` must expose a native handle; on Bevy it is provided via
/// `RawHandleWrapperHolder`.
///
/// # Example
/// ```rust,no_run
/// use bevy::window::RawHandleWrapperHolder;
/// use wde_wgpu::instance::create_instance;
/// 
/// let render = create_instance("demo", Some(&window)).await;
/// assert!(render.data.read().unwrap().surface.is_some());
/// ```
pub async fn create_instance(label: &str, primary_window: Option<&RawHandleWrapperHolder>) -> RenderInstanceData<'static> {
    info!(label, "Creating render instance.");
    let _trace = info_span!("wgpu-instance-create").entered();

    // Set flags
    let mut flags = if cfg!(debug_assertions) {
        wgpu::InstanceFlags::DEBUG
            | wgpu::InstanceFlags::VALIDATION
            | wgpu::InstanceFlags::VALIDATION_INDIRECT_CALL
    } else {
        wgpu::InstanceFlags::DISCARD_HAL_LABELS
    };
    if cfg!(feature = "gpu-debug") {
        flags |= wgpu::InstanceFlags::GPU_BASED_VALIDATION
    }

    // Create wgpu instance
    debug!(label, "Creating wgpu instance.");
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        flags,
        memory_budget_thresholds: MemoryBudgetThresholds::default(),
        backend_options: BackendOptions::default(),
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
        .enumerate_adapters(wgpu::Backends::all())
        .into_iter()
        .find(|a| a.get_info().backend == wgpu::Backend::Vulkan)
        .or_else(|| block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: surface.as_ref(),
            ..Default::default()
        })).ok())
        .expect("Failed to find a suitable GPU adapter.");

    // Check adaptater infos
    let adapter_info = adapter.get_info();
    info!("Using {} adapter {}.", match adapter_info.device_type {
        wgpu::DeviceType::DiscreteGpu => "discrete GPU",
        wgpu::DeviceType::IntegratedGpu => "integrated GPU",
        wgpu::DeviceType::Cpu => "CPU",
        wgpu::DeviceType::VirtualGpu => "virtual GPU",
        wgpu::DeviceType::Other => "other",
    }, adapter_info.name);
    debug!("Adapter info: {:#?}", adapter_info);
    if adapter_info.device_type == wgpu::DeviceType::Cpu {
        warn!("The selected adapter is using a driver that only supports software rendering, this will be very slow.");
    }

    // Set required features
    let required_features = wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
        | Features::PUSH_CONSTANTS;
        
    // Set limits
    let required_limits = Limits {
        max_push_constant_size: 128,
        max_compute_invocations_per_workgroup: 1024,
        ..Default::default()
    };

    // Create device instance and queue
    trace!(label, "Requesting device.");
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some(label), required_features, required_limits,
                memory_hints: wgpu::MemoryHints::Performance,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                trace: wgpu::Trace::Off,
            },
        )
        .await
        .unwrap_or_else(|e| panic!("Failed to create wgpu device: {:?}", e));

    // Log device infos
    debug!("Configured wgpu adapter limits: {:#?}", device.limits());
    debug!("Configured wgpu adapter features: {:#?}", device.features());

    // Handle uncaught device errors
    device.on_uncaptured_error(std::sync::Arc::new(|error| {
        error!("Uncaptured wgpu error: {:?}", error);
    }));

    // Panic room
    device.set_device_lost_callback(|reason, message| {
        match reason {
            wgpu::DeviceLostReason::Destroyed => error!("The GPU device was destroyed: {}", message),
            wgpu::DeviceLostReason::Unknown => error!("The GPU device was lost for an unknown reason: {}", message),
        }
    });

    // Return instance
    RenderInstanceData {
        device,
        queue,
        surface,
        adapter,
        instance,
        surface_config: None
    }
}

/// Configure a surface for presentation once the window size is known.
///
/// Call this once at startup and again whenever you handle a resize event.
///
/// # Example
/// ```rust,no_run
/// use wde_wgpu::instance::{setup_surface, PresentMode};
/// 
/// let config = setup_surface(label, size, device, surface, adapter, PresentMode::Fifo);
/// println!("configured swapchain {}x{}", config.width, config.height);
/// ```
pub fn setup_surface(label: &str, size: (u32, u32), device: &Device, surface: &Surface, adapter: &wgpu::Adapter, present_mode: PresentMode) -> SurfaceConfiguration {
    debug!(label, "Configuring surface.");

    // Retrieve surface format (sRGB if possible)
    let surface_caps = surface.get_capabilities(adapter);
    debug!("Surface capabilities: {:#?}", surface_caps);
    let surface_format = surface_caps.formats.iter()
        .copied()
        .find(|f| f.is_srgb())
        .unwrap_or(surface_caps.formats[0]);

    // Set texture usage
    let mut usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
    if cfg!(debug_assertions) {
        usage |= wgpu::TextureUsages::COPY_SRC;
    }

    // Set surface configuration
    let surface_config = wgpu::SurfaceConfiguration {
        usage,
        format: surface_format,
        width: size.0,
        height: size.1,
        present_mode,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2
    };
    surface.configure(device, &surface_config);

    surface_config
}

/// Acquire the next surface texture or signal that the surface needs reconfiguration.
///
/// Returns a [`RenderEvent::Redraw`] with a ready-to-use texture view when successful, or
/// [`RenderEvent::Resize`] when the surface was lost/outdated. Forward errors like
/// `OutOfMemory` as `RenderEvent::None` while logging.
///
/// # Example
/// ```rust,no_run
/// use wde_wgpu::instance::{get_current_texture, RenderEvent};
/// 
/// match get_current_texture(surface, config) {
///     RenderEvent::Redraw(frame) => {
///         // frame.texture -> call present after encoding work
///         // frame.view -> attach to render passes
///         drop(frame);
///     }
///     RenderEvent::Resize => println!("surface needs resize"),
///     RenderEvent::None => println!("skipping frame"),
/// }
/// ```
pub fn get_current_texture(surface: &Surface, surface_config: &SurfaceConfiguration) -> RenderEvent {
    event!(Level::TRACE, "Getting current texture.");

    // Get current texture
    let _get_current_texture = info_span!("acquire_texture").entered();
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
                    wgpu::TextureFormat::Bgra8Unorm => Some(wgpu::TextureFormat::Bgra8Unorm),
                    wgpu::TextureFormat::Rgba8Unorm => Some(wgpu::TextureFormat::Rgba8Unorm),
                    _ => unreachable!("Unsupported swapchain format: {:?}", surface_config.format)
                },
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
                usage: None
            });
            let cur_render = RenderTexture {
                texture: surface_texture,
                view: render_view
            };
            RenderEvent::Redraw(cur_render)
        }
        // Surface lost or outdated (minimized or moved to another screen)
        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
            RenderEvent::Resize
        },
        // System out of memory
        Err(wgpu::SurfaceError::OutOfMemory) => {
            error!("System out of memory.");
            RenderEvent::None
        }
        // Timeout of the surface
        Err(wgpu::SurfaceError::Timeout) => {
            warn!("Timeout of the surface.");
            RenderEvent::None
        }
        // Other errors
        Err(e) => {
            error!("Failed to acquire next swapchain texture: {:?}.", e);
            RenderEvent::None
        }
    }
}

/// Present the rendered frame to the swapchain.
///
/// Call this after encoding commands that render into the acquired surface texture.
pub fn present(surface_texture: SurfaceTexture) -> Result<(), RenderError> {
    event!(Level::TRACE, "Presenting render texture.");
    surface_texture.present();
    Ok(())
}

/// Reconfigure a surface after a window resize or when the swapchain is lost.
///
/// The caller must also update `surface_config.width/height` before invoking.
pub fn resize(device: &Device, surface: &Surface, surface_config: &SurfaceConfiguration) {
    event!(Level::TRACE, "Resizing surface to {}x{}.", surface_config.width, surface_config.height);
    surface.configure(device, surface_config);
}
