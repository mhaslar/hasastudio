use crate::{GpuError, FRAME_USAGES, WORKING_FORMAT};
use crossbeam_queue::ArrayQueue;
use rezie_core::FrameTime;
use std::{
    marker::PhantomData,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

/// Dimensions and format of a reusable working texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameKey {
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
}
impl FrameKey {
    /// Validate dimensions without allocating any GPU resources.
    pub fn new(width: u32, height: u32) -> Result<Self, GpuError> {
        if width == 0
            || height == 0
            || u64::from(width)
                .checked_mul(u64::from(height))
                .and_then(|n| n.checked_mul(8))
                .is_none()
        {
            return Err(GpuError::Dimensions { width, height });
        }
        Ok(Self {
            width,
            height,
            format: WORKING_FORMAT,
        })
    }
    /// Texture width in pixels.
    pub fn width(self) -> u32 {
        self.width
    }
    /// Texture height in pixels.
    pub fn height(self) -> u32 {
        self.height
    }
    /// Always the linear premultiplied working format.
    pub fn format(self) -> wgpu::TextureFormat {
        self.format
    }
    fn bytes(self) -> u64 {
        u64::from(self.width) * u64::from(self.height) * 8
    }
}

/// Texture/view creation-call counters, not driver-private allocation measurements.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AllocationCounts {
    /// Calls creating frame textures.
    pub textures: u64,
    /// Calls creating their reusable views.
    pub views: u64,
}
struct Slot {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    owners: AtomicUsize,
}
struct Bucket {
    key: FrameKey,
    slots: Box<[Slot]>,
    free: ArrayQueue<usize>,
    broken: AtomicBool,
}
impl Bucket {
    fn bytes(&self) -> u64 {
        self.key.bytes() * self.slots.len() as u64
    }
}

/// Control-side resource owner. Keep it alive until all worker readers/frames stop.
/// Growth publishes a new reader; existing leases keep the old bucket alive.
/// Retired resources are collected only here, never by active workers.
///
/// ```compile_fail
/// fn cannot_move_allocator(pool: rezie_gpu::FramePool) {
///     std::thread::spawn(move || drop(pool));
/// }
/// ```
pub struct FramePool {
    device: wgpu::Device,
    budget: u64,
    active: Vec<Arc<Bucket>>,
    retired: Vec<Arc<Bucket>>,
    counts: AllocationCounts,
    _control_thread: PhantomData<Rc<()>>,
}
impl FramePool {
    /// Create a control-thread-affine allocator with an explicit GPU texture-byte budget.
    pub fn new(device: wgpu::Device, budget_bytes: u64) -> Self {
        Self {
            device,
            budget: budget_bytes,
            active: Vec::new(),
            retired: Vec::new(),
            counts: AllocationCounts::default(),
            _control_thread: PhantomData,
        }
    }
    /// Current cumulative creation-call counts; inspect on the control side.
    pub fn allocations(&self) -> AllocationCounts {
        self.counts
    }
    /// Drop retired buckets after every external reader/frame has released them.
    pub fn collect_retired(&mut self) {
        self.retired.retain(|b| Arc::strong_count(b) != 1);
    }
    /// Reserve on the control side, then publish the returned worker reader.
    /// Does not shrink existing buckets. Old readers remain valid across growth.
    pub async fn reserve(
        &mut self,
        key: FrameKey,
        capacity: usize,
    ) -> Result<FrameReader, GpuError> {
        if !(1..=1024).contains(&capacity) {
            return Err(GpuError::Capacity(capacity));
        }
        let max = self.device.limits().max_texture_dimension_2d;
        if key.width > max || key.height > max {
            return Err(GpuError::Dimensions {
                width: key.width,
                height: key.height,
            });
        }
        self.collect_retired();
        let old = self.active.iter().position(|b| b.key == key);
        if let Some(i) = old {
            if self.active[i].slots.len() >= capacity {
                return Ok(FrameReader {
                    bucket: self.active[i].clone(),
                });
            }
        }
        let used: u64 = self
            .active
            .iter()
            .chain(&self.retired)
            .map(|b| b.bytes())
            .sum();
        let requested = key
            .bytes()
            .checked_mul(capacity as u64)
            .ok_or(GpuError::Budget {
                requested: u64::MAX,
                available: self.budget.saturating_sub(used),
            })?;
        let available = self.budget.saturating_sub(used);
        if requested > available {
            return Err(GpuError::Budget {
                requested,
                available,
            });
        }
        let free = ArrayQueue::new(capacity);
        for i in 0..capacity {
            free.push(i).map_err(|_| GpuError::Ownership)?;
        }
        let mut slots = Vec::with_capacity(capacity);
        self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        for _ in 0..capacity {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("FramePool working texture"),
                size: wgpu::Extent3d {
                    width: key.width,
                    height: key.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: key.format,
                usage: FRAME_USAGES,
                view_formats: &[],
            });
            self.counts.textures += 1;
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.counts.views += 1;
            slots.push(Slot {
                texture,
                view,
                owners: AtomicUsize::new(0),
            });
        }
        let validation = self.device.pop_error_scope().await;
        let memory = self.device.pop_error_scope().await;
        if let Some(error) = validation.or(memory) {
            return Err(GpuError::Device(error.to_string()));
        }
        let bucket = Arc::new(Bucket {
            key,
            slots: slots.into_boxed_slice(),
            free,
            broken: AtomicBool::new(false),
        });
        if let Some(i) = old {
            self.retired
                .push(std::mem::replace(&mut self.active[i], bucket.clone()));
        } else {
            self.active.push(bucket.clone());
        }
        Ok(FrameReader { bucket })
    }
}

/// Sendable worker access to one preallocated size/format bucket. Cloning allocates nothing.
#[derive(Clone)]
pub struct FrameReader {
    bucket: Arc<Bucket>,
}
impl FrameReader {
    /// Number of preallocated slots in this reader's bucket.
    pub fn capacity(&self) -> usize {
        self.bucket.slots.len()
    }
    /// Free-slot count; a concurrent acquisition may change it immediately.
    pub fn available(&self) -> usize {
        self.bucket.free.len()
    }
    /// Try once; exhaustion returns None and never waits or grows the pool.
    pub fn try_acquire(&self, time: FrameTime) -> Result<Option<Frame>, GpuError> {
        if self.bucket.broken.load(Ordering::Acquire) {
            return Err(GpuError::Ownership);
        }
        let Some(index) = self.bucket.free.pop() else {
            return Ok(None);
        };
        if self.bucket.slots[index].owners.swap(1, Ordering::AcqRel) != 0 {
            self.bucket.broken.store(true, Ordering::Release);
            return Err(GpuError::Ownership);
        }
        Ok(Some(Frame {
            bucket: self.bucket.clone(),
            index,
            time,
        }))
    }
}

/// A GPU working-frame lease carrying programme time. No CPU pixel payload.
/// Retain this lease until every GPU use completes, not just command recording.
/// A texture/view handle copied out of this lease does not keep its pool slot reserved.
pub struct Frame {
    bucket: Arc<Bucket>,
    index: usize,
    time: FrameTime,
}
impl Frame {
    /// Share the lease without allocating; only the final owner returns its slot.
    pub fn try_clone(&self) -> Result<Self, GpuError> {
        self.bucket.slots[self.index]
            .owners
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| GpuError::ReferenceOverflow)?;
        Ok(Self {
            bucket: self.bucket.clone(),
            index: self.index,
            time: self.time,
        })
    }
    /// Programme index and exact PTS attached by the producer.
    pub fn time(&self) -> FrameTime {
        self.time
    }
    /// Dimensions and working format.
    pub fn key(&self) -> FrameKey {
        self.bucket.key
    }
    /// GPU texture for ingest/egress. Keep the lease through GPU completion.
    pub fn texture(&self) -> &wgpu::Texture {
        &self.bucket.slots[self.index].texture
    }
    /// Preallocated sampling/render view. Keep the lease through GPU completion.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.bucket.slots[self.index].view
    }
}
impl Drop for Frame {
    fn drop(&mut self) {
        let before = self.bucket.slots[self.index]
            .owners
            .fetch_sub(1, Ordering::AcqRel);
        if before == 0 || (before == 1 && self.bucket.free.push(self.index).is_err()) {
            self.bucket.broken.store(true, Ordering::Release);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frame_keys_reject_invalid_sizes_and_have_one_working_format() {
        assert!(FrameKey::new(0, 1080).is_err());
        assert!(FrameKey::new(1920, 0).is_err());
        assert!(FrameKey::new(u32::MAX, u32::MAX).is_err());
        let key = FrameKey::new(1920, 1080).unwrap();
        assert_eq!(key.format(), WORKING_FORMAT);
        assert_eq!(key.bytes(), 16_588_800);
    }
}
