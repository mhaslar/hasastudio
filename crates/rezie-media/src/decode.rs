//! Decode state machine; no programme clock, GPU allocations or colour transforms.
use crate::{initialize, MediaError};
use ffmpeg_next::{self as ffmpeg, codec, ffi, format, media, util::frame::video::Video};
use serde::Serialize;
use std::{
    ffi::{CStr, CString},
    marker::PhantomData,
    path::{Path, PathBuf},
    ptr,
    rc::Rc,
};

/// Hardware selection policy. Diagnostics can require actual hardware execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeMode {
    /// Try the platform hardware path, then reopen in software on failure.
    Auto,
    /// Explicit software path, including libdav1d for AV1.
    Software,
    /// Fail if a hardware context/frame cannot be produced; never silently fall back.
    RequireHardware,
}

/// Observed selection, updated after actual frame reception.
#[derive(Debug, Clone, Serialize)]
pub struct DecodeStatus {
    /// Selected FFmpeg decoder name; insufficient alone to prove hardware use.
    pub decoder: String,
    /// Requested native device backend.
    pub hardware_device: Option<String>,
    /// A decoded frame actually carried AVHWFramesContext.
    pub hardware_frame_context_observed: bool,
    /// Format of the native hardware frame before transfer.
    pub hardware_pixel_format: Option<String>,
    /// Device type read from the decoded frame’s actual AVHWDeviceContext.
    pub observed_hardware_device: Option<String>,
    /// Actual Apple session property, not inferred from a VideoToolbox frame.
    pub videotoolbox_using_hardware: Option<bool>,
    /// Hardware disable override was explicitly active.
    pub environment_disabled_hardware: bool,
    /// Why software was selected after hardware was attempted.
    pub fallback_reason: Option<String>,
}

/// Borrowed decoder staging data. It is never published as a programme frame.
/// Upload/conversion into FramePool working textures follows on the media worker.
pub struct DecodedPicture<'a> {
    pub(crate) frame: &'a Video,
    /// Zero-based display index, including frames released while draining EOF.
    pub index: u64,
    /// Container/decoder PTS, without guessed replacement timestamps.
    pub pts: Option<i64>,
    /// Rational time base for PTS, numerator then denominator.
    pub time_base: (i32, i32),
    /// Native frame duration in the same time base (zero means unspecified).
    pub duration: i64,
}

impl DecodedPicture<'_> {
    /// Unscaled decoded dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.frame.width(), self.frame.height())
    }
    /// Borrow native staging data until the next decode call; never a CPU copy.
    pub fn native(&self) -> &Video {
        self.frame
    }
}

struct State {
    // Drop the codec (including its worker threads) before callback storage.
    decoder: ffmpeg::decoder::Video,
    _selection: Option<Box<ffi::AVPixelFormat>>,
    input: format::context::Input,
    stream: usize,
    time_base: (i32, i32),
    decoded: Video,
    transferred: Video,
    flushing: bool,
    finished: bool,
    status: DecodeStatus,
}

/// A file decoder owned and driven by one media thread. Not Send or Sync.
pub struct FileDecoder {
    state: State,
    path: PathBuf,
    mode: DecodeMode,
    delivered: u64,
    _thread_affine: PhantomData<Rc<()>>,
}

fn error(path: &Path, operation: &'static str, detail: impl ToString) -> MediaError {
    MediaError::Input {
        path: path.display().to_string(),
        operation,
        detail: detail.to_string(),
    }
}

fn pixel_name(pixel: ffi::AVPixelFormat) -> String {
    // SAFETY: FFmpeg returns static storage or null for an unknown format.
    unsafe {
        let name = ffi::av_get_pix_fmt_name(pixel);
        if name.is_null() {
            format!("unknown({})", pixel as i32)
        } else {
            CStr::from_ptr(name).to_string_lossy().into_owned()
        }
    }
}

unsafe extern "C" fn select_hardware(
    context: *mut ffi::AVCodecContext,
    mut formats: *const ffi::AVPixelFormat,
) -> ffi::AVPixelFormat {
    // SAFETY: FFmpeg calls with its live codec and a NONE-terminated format list.
    // opaque points to stable Box storage retained until after codec destruction.
    unsafe {
        let desired = *(*context).opaque.cast::<ffi::AVPixelFormat>();
        while *formats != ffi::AVPixelFormat::AV_PIX_FMT_NONE {
            if *formats == desired {
                return desired;
            }
            formats = formats.add(1);
        }
    }
    // Never select a software format while reporting a hardware attempt as success.
    ffi::AVPixelFormat::AV_PIX_FMT_NONE
}

fn empty_frame(path: &Path) -> Result<Video, MediaError> {
    let frame = Video::empty();
    // SAFETY: inspecting the wrapper's pointer does not dereference an allocation.
    if unsafe { frame.as_ptr() }.is_null() {
        return Err(error(path, "allocate decoder staging", "out of memory"));
    }
    Ok(frame)
}

impl State {
    fn open(path: &Path, hardware: bool, disabled: bool) -> Result<Self, MediaError> {
        // ffmpeg-next's path conversion expects UTF-8 and no embedded NUL.
        let text = path
            .to_str()
            .ok_or_else(|| error(path, "open", "path is not UTF-8"))?;
        CString::new(text).map_err(|e| error(path, "open", e))?;
        let mut options = ffmpeg::Dictionary::new();
        options.set("protocol_whitelist", "file");
        let input = format::input_with_dictionary(&text, options)
            .map_err(|e| error(path, "demux open", e))?;
        let stream = input
            .streams()
            .best(media::Type::Video)
            .ok_or_else(|| error(path, "select stream", "no video stream found"))?;
        let index = stream.index();
        let time = stream.time_base();
        if time.numerator() <= 0 || time.denominator() <= 0 {
            return Err(error(path, "stream timing", "invalid time base"));
        }
        let parameters = stream.parameters();
        let id = parameters.id();
        let name = match id {
            codec::Id::H264 => "h264",
            codec::Id::HEVC => "hevc",
            codec::Id::VP9 => "vp9",
            codec::Id::AV1 if hardware => "av1",
            codec::Id::AV1 => "libdav1d",
            _ => {
                return Err(error(
                    path,
                    "select decoder",
                    format!("codec {id:?} is outside Phase 1 file decode"),
                ))
            }
        };
        let codec = ffmpeg::decoder::find_by_name(name).ok_or_else(|| {
            error(
                path,
                "select decoder",
                format!("required decoder '{name}' is absent"),
            )
        })?;
        let context = codec::context::Context::from_parameters(parameters)
            .map_err(|e| error(path, "copy stream parameters", e))?;
        let mut decoder = context.decoder();
        decoder.set_packet_time_base(time);
        let mut selection = None;
        let backend = if cfg!(windows) {
            "d3d11va"
        } else if cfg!(target_os = "macos") {
            "videotoolbox"
        } else {
            "vaapi"
        };
        if hardware {
            decoder.set_threading(codec::threading::Config::count(1));
            let backend_c = CString::new(backend).map_err(|e| error(path, "hardware name", e))?;
            // SAFETY: codec is live; hardware configs are immutable library-owned
            // descriptors. Device ownership is handed to the codec on success.
            unsafe {
                let kind = ffi::av_hwdevice_find_type_by_name(backend_c.as_ptr());
                let mut config_index = 0;
                let desired = loop {
                    let config = ffi::avcodec_get_hw_config(codec.as_ptr(), config_index);
                    if config.is_null() {
                        return Err(error(
                            path,
                            "hardware selection",
                            format!("{name} has no {backend} device-context configuration"),
                        ));
                    }
                    if (*config).device_type == kind
                        && ((*config).methods & ffi::AV_CODEC_HW_CONFIG_METHOD_HW_DEVICE_CTX as i32)
                            != 0
                    {
                        break (*config).pix_fmt;
                    }
                    config_index += 1;
                };
                let mut device = ptr::null_mut();
                let result =
                    ffi::av_hwdevice_ctx_create(&mut device, kind, ptr::null(), ptr::null_mut(), 0);
                if result < 0 {
                    return Err(error(
                        path,
                        "create hardware device",
                        format!("{backend}: {}", ffmpeg::Error::from(result)),
                    ));
                }
                let mut value = Box::new(desired);
                (*decoder.as_mut_ptr()).hw_device_ctx = device;
                (*decoder.as_mut_ptr()).opaque = (&mut *value as *mut ffi::AVPixelFormat).cast();
                (*decoder.as_mut_ptr()).get_format = Some(select_hardware);
                selection = Some(value);
            }
        }
        let decoder = decoder
            .open_as(codec)
            .and_then(|opened| opened.video())
            .map_err(|e| error(path, "open decoder", format!("{name}: {e}")))?;
        let decoded = empty_frame(path)?;
        let transferred = empty_frame(path)?;
        Ok(Self {
            decoder,
            _selection: selection,
            input,
            stream: index,
            time_base: (time.numerator(), time.denominator()),
            decoded,
            transferred,
            flushing: false,
            finished: false,
            status: DecodeStatus {
                decoder: name.into(),
                hardware_device: hardware.then(|| backend.into()),
                hardware_frame_context_observed: false,
                hardware_pixel_format: None,
                observed_hardware_device: None,
                videotoolbox_using_hardware: None,
                environment_disabled_hardware: disabled,
                fallback_reason: None,
            },
        })
    }

    fn next(&mut self, path: &Path) -> Result<bool, MediaError> {
        if self.finished {
            return Ok(false);
        }
        loop {
            match self.decoder.receive_frame(&mut self.decoded) {
                Ok(()) => {
                    // SAFETY: receive_frame succeeded and filled a live AVFrame.
                    let hardware = unsafe { !(*self.decoded.as_ptr()).hw_frames_ctx.is_null() };
                    if hardware != self.status.hardware_device.is_some() {
                        return Err(error(
                            path,
                            "verify decode path",
                            "hardware frame context disagrees with selected path",
                        ));
                    }
                    if hardware {
                        // SAFETY: the successful hardware frame owns its frames-context
                        // reference, which in turn owns the documented device context.
                        let observed = unsafe {
                            let context = &*((*self.decoded.as_ptr())
                                .hw_frames_ctx
                                .as_ref()
                                .ok_or_else(|| {
                                    error(path, "hardware frame context", "missing reference")
                                })?
                                .data
                                .cast::<ffi::AVHWFramesContext>());
                            if context.device_ctx.is_null() {
                                return Err(error(
                                    path,
                                    "hardware device context",
                                    "missing native device",
                                ));
                            }
                            let kind = (*context.device_ctx).type_;
                            let name = ffi::av_hwdevice_get_type_name(kind);
                            if name.is_null() {
                                return Err(error(
                                    path,
                                    "hardware device context",
                                    "unknown device type",
                                ));
                            }
                            CStr::from_ptr(name).to_string_lossy().into_owned()
                        };
                        if Some(&observed) != self.status.hardware_device.as_ref() {
                            return Err(error(
                                path,
                                "hardware device context",
                                "actual device differs from requested backend",
                            ));
                        }
                        #[cfg(target_os = "macos")]
                        {
                            // SAFETY: the optional accessor lives inside the loaded
                            // FFmpeg and reads its own context layout. The codec is
                            // live, owned by this thread; the handle outlives the call.
                            let active = unsafe {
                                let library = libloading::os::unix::Library::this();
                                type Probe =
                                    unsafe extern "C" fn(*const ffi::AVCodecContext) -> i32;
                                let probe = library
                                    .get::<Probe>(b"av_rezie_videotoolbox_uses_hardware\0")
                                    .map_err(|e| {
                                        error(path, "VideoToolbox hardware verification", e)
                                    })?;
                                probe(self.decoder.as_ptr())
                            };
                            if active != 1 {
                                return Err(error(
                                    path,
                                    "VideoToolbox hardware verification",
                                    format!("actual session hardware property returned {active}"),
                                ));
                            }
                            self.status.videotoolbox_using_hardware = Some(true);
                        }
                        self.status.observed_hardware_device = Some(observed);
                        self.status.hardware_frame_context_observed = true;
                        self.status.hardware_pixel_format =
                            Some(pixel_name(self.decoded.format().into()));
                        // SAFETY: source/destination are live distinct AVFrames. Unref
                        // releases the previous transfer; FFmpeg allocates native staging.
                        // Copy props retains exact PTS and colour metadata after transfer.
                        unsafe {
                            ffi::av_frame_unref(self.transferred.as_mut_ptr());
                            let result = ffi::av_hwframe_transfer_data(
                                self.transferred.as_mut_ptr(),
                                self.decoded.as_ptr(),
                                0,
                            );
                            if result < 0 {
                                return Err(error(
                                    path,
                                    "hardware frame transfer",
                                    ffmpeg::Error::from(result),
                                ));
                            }
                            let result = ffi::av_frame_copy_props(
                                self.transferred.as_mut_ptr(),
                                self.decoded.as_ptr(),
                            );
                            if result < 0 {
                                return Err(error(
                                    path,
                                    "copy hardware frame metadata",
                                    ffmpeg::Error::from(result),
                                ));
                            }
                        }
                    }
                    return Ok(true);
                }
                Err(ffmpeg::Error::Eof) => {
                    self.finished = true;
                    return Ok(false);
                }
                Err(ffmpeg::Error::Other { errno }) if errno == ffmpeg::error::EAGAIN => {}
                Err(e) => return Err(error(path, "receive decoded picture", e)),
            }
            if self.flushing {
                return Err(error(
                    path,
                    "drain decoder",
                    "unexpected EAGAIN after EOF was accepted",
                ));
            }
            let mut packet = ffmpeg::Packet::empty();
            loop {
                match packet.read(&mut self.input) {
                    Ok(()) if packet.stream() == self.stream => {
                        self.decoder
                            .send_packet(&packet)
                            .map_err(|e| error(path, "send video packet", e))?;
                        break;
                    }
                    Ok(()) => {
                        packet = ffmpeg::Packet::empty();
                        continue;
                    }
                    Err(ffmpeg::Error::Eof) => {
                        self.decoder
                            .send_eof()
                            .map_err(|e| error(path, "send decoder EOF", e))?;
                        self.flushing = true;
                        break;
                    }
                    Err(e) => return Err(error(path, "read packet", e)),
                }
            }
        }
    }
}

impl FileDecoder {
    /// Open a local file. REZIE_DISABLE_HW_DECODE=1 forces software even in Auto.
    /// A conflicting RequireHardware diagnostic fails explicitly.
    pub fn open(path: impl AsRef<Path>, mode: DecodeMode) -> Result<Self, MediaError> {
        initialize()?;
        let path = path.as_ref().to_path_buf();
        let disabled = match std::env::var("REZIE_DISABLE_HW_DECODE") {
            Err(std::env::VarError::NotPresent) => false,
            Ok(value) if value == "1" => true,
            Ok(value) if value == "0" => false,
            _ => {
                return Err(error(
                    &path,
                    "hardware override",
                    "REZIE_DISABLE_HW_DECODE must be 0 or 1",
                ))
            }
        };
        if disabled && mode == DecodeMode::RequireHardware {
            return Err(error(
                &path,
                "hardware override",
                "hardware is required but explicitly disabled",
            ));
        }
        let hardware = !disabled && mode != DecodeMode::Software;
        let state = match State::open(&path, hardware, disabled) {
            Ok(state) => state,
            Err(e) if hardware && mode == DecodeMode::Auto => {
                let mut state = State::open(&path, false, disabled)?;
                state.status.fallback_reason = Some(e.to_string());
                state
            }
            Err(e) => return Err(e),
        };
        Ok(Self {
            state,
            path,
            mode,
            delivered: 0,
            _thread_affine: PhantomData,
        })
    }

    /// Actual selection and observations so far; hardware proof requires a frame.
    pub fn status(&self) -> &DecodeStatus {
        &self.state.status
    }

    /// Receive the next display-order picture, draining delayed frames at EOF.
    /// Borrowed staging storage stays valid until the next mutable decoder call.
    pub fn next_picture(&mut self) -> Result<Option<DecodedPicture<'_>>, MediaError> {
        let available = match self.state.next(&self.path) {
            Ok(available) => available,
            Err(e)
                if self.mode == DecodeMode::Auto && self.state.status.hardware_device.is_some() =>
            {
                let mut software = State::open(&self.path, false, false)?;
                software.status.fallback_reason = Some(e.to_string());
                // Rare error recovery, off the composite thread. Do not re-emit
                // already delivered frames or guess a seek position from DTS.
                for _ in 0..self.delivered {
                    if !software.next(&self.path)? {
                        return Err(error(
                            &self.path,
                            "software recovery",
                            "file ended before prior display position",
                        ));
                    }
                }
                self.state = software;
                self.state.next(&self.path)?
            }
            Err(e) => return Err(e),
        };
        if !available {
            return Ok(None);
        }
        let frame = if self.state.status.hardware_device.is_some() {
            &self.state.transferred
        } else {
            &self.state.decoded
        };
        let picture = DecodedPicture {
            frame,
            index: self.delivered,
            pts: frame.pts(),
            time_base: self.state.time_base,
            duration: frame.packet().duration,
        };
        self.delivered += 1;
        Ok(Some(picture))
    }
}
