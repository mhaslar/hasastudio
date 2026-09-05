//! GPU device ownership and preallocated working-frame leases.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod pool;
pub use pool::{AllocationCounts, Frame, FrameKey, FramePool, FrameReader};

/// The sole internal working-frame format: linear BT.709, premultiplied alpha.
pub const WORKING_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
/// Usages required by ingest, compositing, preview and egress.
pub const FRAME_USAGES: wgpu::TextureUsages = wgpu::TextureUsages::TEXTURE_BINDING
    .union(wgpu::TextureUsages::STORAGE_BINDING)
    .union(wgpu::TextureUsages::RENDER_ATTACHMENT)
    .union(wgpu::TextureUsages::COPY_SRC)
    .union(wgpu::TextureUsages::COPY_DST);

/// Actionable setup or pool error; hot-path variants allocate no error strings.
#[derive(Debug, thiserror::Error)]
pub enum GpuError {
    /// Adapter/device request or scoped GPU resource error.
    #[error("GPU setup failed: {0}")]
    Device(String),
    /// Invalid working-frame dimensions.
    #[error("invalid working-frame dimensions {width}x{height}")]
    Dimensions {
        /// Requested width.
        width: u32,
        /// Requested height.
        height: u32,
    },
    /// Invalid reservation size.
    #[error("frame pool reservation requires 1–1024 slots; requested {0}")]
    Capacity(usize),
    /// Reservation exceeds the explicit control-side byte budget.
    #[error("frame pool needs {requested} more bytes, but only {available} remain")]
    Budget {
        /// Additional GPU texture bytes requested.
        requested: u64,
        /// Remaining budget, including retained old buckets.
        available: u64,
    },
    /// Internal free-list/refcount invariant failed; stop using this reader.
    #[error("frame pool ownership invariant failed")]
    Ownership,
    /// A pathological number of shared owners cannot be represented.
    #[error("frame lease reference count overflow")]
    ReferenceOverflow,
}

/// Native GPU handles owned by the GPU subsystem, reusable by a future GUI setup.
#[derive(Clone)]
pub struct GpuContext {
    /// Instance used to create adapters and surfaces.
    pub instance: wgpu::Instance,
    /// Selected native adapter; record its identity with hardware evidence.
    pub adapter: wgpu::Adapter,
    /// Shared device; all frame resource creation belongs to FramePool.
    pub device: wgpu::Device,
    /// Shared submission queue; leases must survive completion of their GPU uses.
    pub queue: wgpu::Queue,
}

impl GpuContext {
    /// Request the platform's specified backend with portable limits and working-format support.
    pub async fn request() -> Result<Self, GpuError> {
        let backends = if cfg!(target_os = "macos") {
            wgpu::Backends::METAL
        } else if cfg!(target_os = "windows") {
            wgpu::Backends::DX12
        } else {
            wgpu::Backends::VULKAN
        };
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| GpuError::Device(e.to_string()))?;
        let format = adapter.get_texture_format_features(WORKING_FORMAT);
        if !format.allowed_usages.contains(FRAME_USAGES) {
            return Err(GpuError::Device(format!(
                "adapter '{}' lacks Rgba16Float usages {:?}",
                adapter.get_info().name,
                FRAME_USAGES - format.allowed_usages
            )));
        }
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Rezie shared device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| GpuError::Device(e.to_string()))?;
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }
}
