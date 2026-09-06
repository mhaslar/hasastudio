use super::{FrameKey, FramePool};
use crate::GpuError;
use rezie_core::FrameTime;
use std::{sync::mpsc, time::Duration};

/// Exact shader source, exposed so diagnostic reports can identify what ran.
pub const COLOUR_SHADER: &str = include_str!("colour.wgsl");

/// Diagnostic/egress readback only; never an engine frame or preview transport.
pub struct ColourReadback {
    /// Straight-alpha sRGB RGBA8 bytes ready for PNG export.
    pub png_rgba: Vec<u8>,
    /// Raw linear premultiplied Rgba16Float result expanded for numerical inspection.
    pub linear_rgba: Vec<[f32; 4]>,
}

impl FramePool {
    /// Run a small, blocking colour diagnostic on the control thread only.
    ///
    /// Allocates diagnostic GPU resources here, performs ingest/over/egress and
    /// waits for completion before releasing frame leases. Input is tightly
    /// packed straight-alpha sRGB RGBA8. This is not a streaming submission API.
    pub async fn check_colour(
        &mut self,
        queue: &wgpu::Queue,
        key: FrameKey,
        png_rgba: &[u8],
        background: [u8; 4],
    ) -> Result<ColourReadback, GpuError> {
        let bytes = u64::from(key.width()) * u64::from(key.height()) * 4;
        if bytes != png_rgba.len() as u64 || bytes > 16 * 1024 * 1024 {
            return Err(GpuError::Diagnostic(format!(
                "{}x{} needs {bytes} RGBA bytes (diagnostic limit 16 MiB), got {}",
                key.width(),
                key.height(),
                png_rgba.len()
            )));
        }
        let row_pitch = (u64::from(key.width()) * 8).div_ceil(256) * 256;
        let raw_size = row_pitch * u64::from(key.height());
        let buffer_bytes = bytes * 3 + raw_size + 16;
        let reader = self.reserve(key, 3).await?;
        let used: u64 = self
            .active
            .iter()
            .chain(&self.retired)
            .map(|b| b.bytes())
            .sum();
        let available = self.budget.saturating_sub(used);
        if buffer_bytes > available {
            return Err(GpuError::Budget {
                requested: buffer_bytes,
                available,
            });
        }
        let time = FrameTime {
            index: 0,
            pts: Duration::ZERO,
        };
        let fg = reader.try_acquire(time)?.ok_or(GpuError::Ownership)?;
        let bg = reader.try_acquire(time)?.ok_or(GpuError::Ownership)?;
        let out = reader.try_acquire(time)?.ok_or(GpuError::Ownership)?;
        self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let result = (|| {
            // Every resource is created under this exclusive, control-thread pool borrow.
            let buffer = |label, size, usage, contents: Option<&[u8]>| {
                let b = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size,
                    usage,
                    mapped_at_creation: contents.is_some(),
                });
                if let Some(data) = contents {
                    b.slice(..).get_mapped_range_mut().copy_from_slice(data);
                    b.unmap();
                }
                b
            };
            let input = buffer(
                "PNG ingest bytes",
                bytes,
                wgpu::BufferUsages::STORAGE,
                Some(png_rgba),
            );
            let mut background_bytes = [0_u8; 16];
            background_bytes[..4].copy_from_slice(&background);
            let colour = buffer(
                "sRGB colour source",
                16,
                wgpu::BufferUsages::UNIFORM,
                Some(&background_bytes),
            );
            let export = buffer(
                "PNG egress bytes",
                bytes,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                None,
            );
            let png_readback = buffer(
                "PNG diagnostic readback",
                bytes,
                wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                None,
            );
            let raw_readback = buffer(
                "linear diagnostic readback",
                raw_size,
                wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                None,
            );
            let shader = self
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("linear PNG alpha diagnostic"),
                    source: wgpu::ShaderSource::Wgsl(COLOUR_SHADER.into()),
                });
            let pipeline = |entry| {
                self.device
                    .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                        label: Some(entry),
                        layout: None,
                        module: &shader,
                        entry_point: Some(entry),
                        compilation_options: Default::default(),
                        cache: None,
                    })
            };
            let ingest = pipeline("ingest");
            let composite = pipeline("composite");
            let egress = pipeline("egress");
            let group = |pipeline: &wgpu::ComputePipeline, entries: &[wgpu::BindGroupEntry<'_>]| {
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("colour diagnostic resources"),
                    layout: &pipeline.get_bind_group_layout(0),
                    entries,
                })
            };
            let ingest_group = group(
                &ingest,
                &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(fg.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(bg.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: colour.as_entire_binding(),
                    },
                ],
            );
            let composite_group = group(
                &composite,
                &[
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(fg.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(bg.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(out.view()),
                    },
                ],
            );
            let egress_group = group(
                &egress,
                &[
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: wgpu::BindingResource::TextureView(out.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: 8,
                        resource: export.as_entire_binding(),
                    },
                ],
            );
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("control-side colour check"),
                });
            for (pipeline, group) in [
                (&ingest, &ingest_group),
                (&composite, &composite_group),
                (&egress, &egress_group),
            ] {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("colour boundary/composite"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, group, &[]);
                pass.dispatch_workgroups(key.width().div_ceil(8), key.height().div_ceil(8), 1);
            }
            encoder.copy_buffer_to_buffer(&export, 0, &png_readback, 0, bytes);
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: out.texture(),
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &raw_readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(row_pitch as u32),
                        rows_per_image: None,
                    },
                },
                wgpu::Extent3d {
                    width: key.width(),
                    height: key.height(),
                    depth_or_array_layers: 1,
                },
            );
            queue.submit([encoder.finish()]);
            // Blocking wait is intentional here; this function cannot be used on the compositor thread.
            let png_rgba = read_buffer(&self.device, &png_readback)?;
            let raw = read_buffer(&self.device, &raw_readback)?;
            let mut linear_rgba = Vec::with_capacity(png_rgba.len() / 4);
            for row in raw.chunks_exact(row_pitch as usize) {
                for pixel in row[..key.width() as usize * 8].as_chunks::<8>().0 {
                    linear_rgba.push(std::array::from_fn(|i| {
                        half_to_float(u16::from_le_bytes([pixel[i * 2], pixel[i * 2 + 1]]))
                    }));
                }
            }
            Ok(ColourReadback {
                png_rgba,
                linear_rgba,
            })
        })();
        let validation = self.device.pop_error_scope().await;
        let memory = self.device.pop_error_scope().await;
        if let Some(error) = validation.or(memory) {
            return Err(GpuError::Device(format!("colour check: {error}")));
        }
        result
    }
}

fn read_buffer(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Result<Vec<u8>, GpuError> {
    let (send, recv) = mpsc::sync_channel(1);
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = send.send(r);
    });
    device
        .poll(wgpu::PollType::Wait)
        .map_err(|e| GpuError::Diagnostic(format!("wait for GPU: {e}")))?;
    recv.recv_timeout(Duration::from_secs(10))
        .map_err(|e| GpuError::Diagnostic(format!("readback callback: {e}")))?
        .map_err(|e| GpuError::Diagnostic(format!("map readback: {e}")))?;
    let bytes = buffer.slice(..).get_mapped_range().to_vec();
    buffer.unmap();
    Ok(bytes)
}

fn half_to_float(bits: u16) -> f32 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = (bits >> 10) & 31;
    let fraction = f32::from(bits & 1023);
    match exponent {
        0 => sign * fraction * 2_f32.powi(-24),
        31 if fraction == 0.0 => sign * f32::INFINITY,
        31 => f32::NAN,
        _ => sign * (1.0 + fraction / 1024.0) * 2_f32.powi(i32::from(exponent) - 15),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decode_ieee_half_values_including_subnormals() {
        for (bits, expected) in [
            (0, 0.0),
            (0x3c00, 1.0),
            (0xb800, -0.5),
            (1, 2_f32.powi(-24)),
            (0x7bff, 65504.0),
        ] {
            assert_eq!(half_to_float(bits), expected);
        }
        assert_eq!(half_to_float(0x8000).to_bits(), (-0.0_f32).to_bits());
        assert!(half_to_float(0x7c00).is_infinite());
        assert!(half_to_float(0x7e00).is_nan());
    }
}
