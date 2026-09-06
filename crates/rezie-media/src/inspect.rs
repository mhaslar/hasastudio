//! Offline inspection of borrowed decoded samples; never a composite-thread task.
use crate::{DecodedPicture, MediaError};
use ffmpeg_next::ffi;
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Independently comparable metadata and exact decoded component hashes.
#[derive(Debug, Serialize)]
pub struct PictureRecord {
    /// Display-order index.
    pub index: u64,
    /// Observed presentation timestamp; None stays explicit.
    pub pts: Option<i64>,
    /// Timestamp rational, without float conversion.
    pub time_base: (i32, i32),
    /// Observed duration in time-base units.
    pub duration: i64,
    /// Decoded dimensions.
    pub dimensions: (u32, u32),
    /// CPU decoder staging format, after hardware transfer when needed.
    pub pixel_format: String,
    /// Component hashes in descriptor order (Y,U,V or R,G,B, then alpha).
    /// Each sample is an unsigned little-endian u16, with packing/shift removed.
    /// Row padding is excluded, allowing NV12 and planar YUV to compare exactly.
    pub component_sha256: Vec<String>,
    /// Meaningful bits per component; do not equate different bit depths.
    pub component_depth: Vec<u32>,
    /// Matrix coefficients.
    pub colour_space: String,
    /// Full/limited/unspecified range.
    pub colour_range: String,
    /// Original colour primaries.
    pub colour_primaries: String,
    /// Original transfer function.
    pub colour_transfer: String,
    /// Original chroma location.
    pub chroma_location: String,
    /// Alpha component is present in decoded staging format.
    pub has_alpha: bool,
    /// Original interlace flag; this inspector does not deinterlace.
    pub interlaced: bool,
}

impl DecodedPicture<'_> {
    /// Hash actual samples without padding or colour conversion. Diagnostic only.
    pub fn inspect(&self) -> Result<PictureRecord, MediaError> {
        let fail = |detail: &str| {
            MediaError::NativePolicy(format!("inspect decoded picture {}: {detail}", self.index))
        };
        let format: ffi::AVPixelFormat = self.frame.format().into();
        let mut hashes = Vec::new();
        let mut depths = Vec::new();
        // SAFETY: the borrowed AVFrame stays alive and cannot be changed during
        // this call. Descriptor layout defines component offsets/steps/planes.
        // Validate packing and line bounds before reading within each frame row.
        let alpha = unsafe {
            let descriptor = ffi::av_pix_fmt_desc_get(format);
            if descriptor.is_null() {
                return Err(fail("unknown pixel format"));
            }
            let d = &*descriptor;
            if d.flags
                & (ffi::AV_PIX_FMT_FLAG_HWACCEL
                    | ffi::AV_PIX_FMT_FLAG_BITSTREAM
                    | ffi::AV_PIX_FMT_FLAG_PAL
                    | ffi::AV_PIX_FMT_FLAG_FLOAT) as u64
                != 0
            {
                return Err(fail(
                    "inspector requires integer, byte-addressed transferred samples",
                ));
            }
            let frame = &*self.frame.as_ptr();
            for c in 0..usize::from(d.nb_components) {
                let component = d.comp[c];
                if !(1..=16).contains(&component.depth)
                    || component.shift < 0
                    || component.shift + component.depth > 16
                    || component.step <= 0
                    || component.offset < 0
                    || !(0..4).contains(&component.plane)
                {
                    return Err(fail("invalid or unsupported component layout"));
                }
                let plane = component.plane as usize;
                let chroma = (c == 1 || c == 2) && d.flags & ffi::AV_PIX_FMT_FLAG_RGB as u64 == 0;
                let width =
                    self.frame
                        .width()
                        .div_ceil(if chroma { 1 << d.log2_chroma_w } else { 1 })
                        as usize;
                let height =
                    self.frame
                        .height()
                        .div_ceil(if chroma { 1 << d.log2_chroma_h } else { 1 })
                        as usize;
                let bytes = ((component.shift + component.depth + 7) / 8) as usize;
                let stride = frame.linesize[plane] as isize;
                let last_byte = width
                    .checked_sub(1)
                    .and_then(|w| w.checked_mul(component.step as usize))
                    .and_then(|w| w.checked_add(component.offset as usize + bytes));
                if frame.data[plane].is_null()
                    || height == 0
                    || last_byte.is_none_or(|b| b > stride.unsigned_abs())
                {
                    return Err(fail("component exceeds decoded row bounds"));
                }
                let mut hash = Sha256::new();
                for y in 0..height {
                    let row = frame.data[plane].offset(y as isize * stride);
                    for x in 0..width {
                        let p = row.add(x * component.step as usize + component.offset as usize);
                        let value = if bytes == 1 {
                            u16::from(*p)
                        } else if d.flags & ffi::AV_PIX_FMT_FLAG_BE as u64 != 0 {
                            u16::from_be_bytes([*p, *p.add(1)])
                        } else {
                            u16::from_le_bytes([*p, *p.add(1)])
                        };
                        let value =
                            (u32::from(value) >> component.shift) & ((1 << component.depth) - 1);
                        hash.update((value as u16).to_le_bytes());
                    }
                }
                depths.push(component.depth as u32);
                hashes.push(format!("{:x}", hash.finalize()));
            }
            d.flags & ffi::AV_PIX_FMT_FLAG_ALPHA as u64 != 0
        };
        Ok(PictureRecord {
            index: self.index,
            pts: self.pts,
            time_base: self.time_base,
            duration: self.duration,
            dimensions: self.dimensions(),
            pixel_format: format!("{:?}", self.frame.format()),
            component_sha256: hashes,
            component_depth: depths,
            colour_space: format!("{:?}", self.frame.color_space()),
            colour_range: format!("{:?}", self.frame.color_range()),
            colour_primaries: format!("{:?}", self.frame.color_primaries()),
            colour_transfer: format!("{:?}", self.frame.color_transfer_characteristic()),
            chroma_location: format!("{:?}", self.frame.chroma_location()),
            has_alpha: alpha,
            interlaced: self.frame.is_interlaced(),
        })
    }
}
