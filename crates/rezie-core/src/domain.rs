use crate::{FrameRate, FrameTime};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};

/// Configured input identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InputId(pub u32);

/// Mix/effect identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MeId(pub u32);

/// Audio bus identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BusId(pub u32);

/// Configured output identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OutputId(pub u32);

/// Recorder identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecorderId(pub u32);

/// Resource residency; transitions are implemented in consuming phases.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum InputState {
    /// Not open or connected.
    #[default]
    Cold,
    /// Prepared; NDI receives lowest bandwidth for multiview.
    Warm,
    /// Actively decoding or connected at full bandwidth.
    Hot,
}

/// A clean video source reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SourceRef {
    /// No configured input.
    Black,
    /// Configured input.
    Input(InputId),
    /// Clean programme from another mix/effect.
    Me(MeId),
}

/// Programme colour specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColorSpace {
    /// BT.709 primaries; GPU working light is linear.
    Bt709,
}

/// Per-input frame-rate conversion policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum FrameSelection {
    /// Select the frame containing the programme midpoint.
    #[default]
    Nearest,
    /// Opt-in interpolation for file-like sources.
    Blend,
}

/// Requested NDI receive bandwidth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NdiBandwidth {
    /// Warm multiview connection.
    Lowest,
    /// Full-bandwidth Hot connection.
    Highest,
}

/// SRT connection role.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SrtMode {
    /// Connect to a listener.
    Caller,
    /// Accept a caller.
    Listener,
}

/// RTSP media transport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RtspTransport {
    /// Interleaved TCP.
    Tcp,
    /// RTP over UDP.
    Udp,
}

/// Platform-neutral capture target identifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CaptureTarget {
    /// Opaque display identity resolved by capture crate.
    Display(String),
    /// Opaque window identity resolved by capture crate.
    Window(String),
    /// Region in display coordinates.
    Region(Rect),
}

/// Configured source description; no source is opened in Foundation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InputKind {
    /// File video or audio.
    File {
        /// Container path.
        path: PathBuf,
        /// In-point.
        start_at: Duration,
        /// Optional out-point.
        end_at: Option<Duration>,
    },
    /// Still image with native alpha.
    Image {
        /// Image path.
        path: PathBuf,
    },
    /// Image sequence.
    ImageSequence {
        /// Filename pattern.
        pattern: String,
        /// Source rate.
        fps: FrameRate,
    },
    /// Solid colour source configuration.
    Color {
        /// RGBA colour values, not a pixel buffer.
        rgba: [f32; 4],
    },
    /// Runtime-provided NDI source.
    Ndi {
        /// Discovery name.
        source_name: String,
        /// Requested bandwidth.
        bandwidth: NdiBandwidth,
    },
    /// SRT stream.
    Srt {
        /// Endpoint.
        url: String,
        /// Role.
        mode: SrtMode,
        /// Receive latency in milliseconds.
        latency_ms: u32,
        /// Optional encryption secret.
        passphrase: Option<String>,
    },
    /// RTMP pull source.
    Rtmp {
        /// Pull URL.
        url: String,
    },
    /// RTSP source.
    Rtsp {
        /// Pull URL.
        url: String,
        /// Media transport.
        transport: RtspTransport,
    },
    /// Screen or window capture.
    ScreenCapture {
        /// Platform-neutral target.
        target: CaptureTarget,
        /// Include cursor.
        cursor: bool,
    },
    /// HTML data only; helper process is Phase 10.
    Html {
        /// Page URL.
        url: String,
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
        /// Page rate.
        fps: FrameRate,
    },
    /// Audio-only file.
    AudioFile {
        /// Audio path.
        path: PathBuf,
    },
    /// Mix/effect output used as an input.
    MeOutput {
        /// Clean programme reference.
        me: MeId,
    },
}

/// Rectangle in the coordinate system of its owning setting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    /// Left coordinate.
    pub x: f32,
    /// Top coordinate.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

/// Source geometry independent of an overlay assignment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transform2D {
    /// Translation in programme pixels.
    pub position: [f32; 2],
    /// Axis scale factors.
    pub scale: [f32; 2],
    /// Rotation in radians.
    pub rotation: f32,
    /// Normalised anchor point.
    pub anchor: [f32; 2],
}

/// File-like source playback configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaybackSettings {
    /// Start position.
    pub in_point: Duration,
    /// Optional stop position.
    pub out_point: Option<Duration>,
    /// Restart at in-point.
    pub looping: bool,
    /// Start when promoted for use.
    pub autoplay: bool,
    /// Playback speed multiplier.
    pub rate: f64,
}

/// Lift/gamma/gain colour controls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorCorrection {
    /// RGB lift.
    pub lift: [f32; 3],
    /// RGB gamma.
    pub gamma: [f32; 3],
    /// RGB gain.
    pub gain: [f32; 3],
    /// Saturation multiplier.
    pub saturation: f32,
    /// Hue rotation in radians.
    pub hue: f32,
    /// Contrast multiplier.
    pub contrast: f32,
}

/// Chroma key parameters; no keyer executes in Foundation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChromaKey {
    /// Key colour.
    pub color: [f32; 3],
    /// Distance threshold.
    pub similarity: f32,
    /// Edge falloff.
    pub smoothness: f32,
    /// Spill suppression amount.
    pub spill: f32,
    /// Choke/expand amount.
    pub edge: f32,
    /// Display greyscale alpha matte.
    pub preview_matte: bool,
}

/// Field-order override.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Interlacing {
    /// Use source metadata.
    Auto,
    /// Treat as progressive.
    Progressive,
    /// Top field first.
    TopFieldFirst,
    /// Bottom field first.
    BottomFieldFirst,
}

/// Source-relative processing configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputProcessing {
    /// Geometry.
    pub transform: Transform2D,
    /// Source-relative normalised crop.
    pub crop: Option<Rect>,
    /// Colour correction.
    pub color: ColorCorrection,
    /// Optional chroma key.
    pub key: Option<ChromaKey>,
    /// Default is Nearest.
    pub frame_selection: FrameSelection,
    /// Field-order detection policy.
    pub interlacing: Interlacing,
    /// Use one frame per field pair when enabled.
    pub half_rate_deinterlace: bool,
}

/// One input-to-bus routing matrix cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioSend {
    /// Destination.
    pub bus: BusId,
    /// Route enabled.
    pub enabled: bool,
    /// Gain offset in dB.
    pub gain_db: f32,
    /// Follow video visibility.
    pub afv: bool,
    /// Solo on this bus only.
    pub solo: bool,
}

/// One parametric EQ setting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EqBand {
    /// Centre/corner frequency.
    pub frequency_hz: f32,
    /// Band gain.
    pub gain_db: f32,
    /// Quality factor.
    pub q: f32,
}

/// Feed-forward soft-knee compressor configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Compressor {
    /// Threshold.
    pub threshold_db: f32,
    /// Compression ratio.
    pub ratio: f32,
    /// Attack time.
    pub attack_ms: f32,
    /// Release time.
    pub release_ms: f32,
    /// Output gain.
    pub makeup_db: f32,
    /// Knee width.
    pub knee_db: f32,
}

/// Per-input audio settings, without audio processing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputAudio {
    /// Lip-sync delay, 0–1000 ms.
    pub delay_ms: u32,
    /// Pre-EQ gain in dB.
    pub gain_db: f32,
    /// Low shelf, peaking mid, high shelf.
    pub eq: [EqBand; 3],
    /// Optional dynamics.
    pub compressor: Option<Compressor>,
    /// Constant-power pan, -1 to 1.
    pub pan: f32,
    /// Mute all sends.
    pub mute: bool,
    /// Per-bus routing row.
    pub sends: Vec<AudioSend>,
}

/// Configured input plus non-persisted resource state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Input {
    /// Identity.
    pub id: InputId,
    /// Operator label.
    pub name: String,
    /// Source configuration.
    pub kind: InputKind,
    /// File-like playback configuration.
    pub playback: PlaybackSettings,
    /// Processing once before sharing.
    pub process: InputProcessing,
    /// Audio processing and routing.
    pub audio: InputAudio,
    /// Runtime-only resource residency.
    #[serde(skip)]
    pub state: InputState,
}

/// Transition direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Direction {
    /// Left direction.
    Left,
    /// Right direction.
    Right,
    /// Up direction.
    Up,
    /// Down direction.
    Down,
}

/// Transition description; execution begins in later phases.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransitionKind {
    /// Immediate switch.
    Cut,
    /// Dissolve.
    Fade,
    /// Wipe.
    Wipe {
        /// Motion direction.
        direction: Direction,
        /// Edge softness.
        softness: f32,
    },
    /// Slide.
    Slide(Direction),
    /// Alpha stinger configuration.
    Stinger {
        /// Alpha-carrying clip.
        input: InputId,
        /// Cut point in frames.
        cut_frame: u64,
    },
}

/// Transition progress curve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Easing {
    /// Linear progress.
    Linear,
    /// Smooth endpoints.
    EaseInOut,
    /// Cubic control points.
    CustomCubic([f32; 4]),
}

/// Frame-counted transition configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionSettings {
    /// Visual operation.
    pub kind: TransitionKind,
    /// Duration on programme timeline.
    pub duration_frames: u64,
    /// Progress mapping.
    pub easing: Easing,
}

/// Runtime fade-to-black state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FadeToBlackState {
    /// Programme visible.
    Off,
    /// Black opacity during transition.
    Transitioning(f32),
    /// Fully black.
    On,
}

/// Clean programme and preview bus configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MixEffect {
    /// Identity.
    pub id: MeId,
    /// Operator label.
    pub name: String,
    /// Programme source.
    pub program: SourceRef,
    /// Preview source.
    pub preview: SourceRef,
    /// Configured transition.
    pub transition: TransitionSettings,
    /// Progress from programme to preview, 0–1.
    pub transition_state: f32,
    /// Fade-to-black state.
    pub ftb: FadeToBlackState,
}

/// Runtime overlay visibility.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OverlayState {
    /// Disabled.
    Off,
    /// Entering, normalised progress.
    TransitioningIn(f32),
    /// Fully visible.
    On,
    /// Leaving, normalised progress.
    TransitioningOut(f32),
}

/// Overlay audio policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OverlayAudio {
    /// Follow visibility.
    Follow,
    /// No overlay audio.
    Ignore,
    /// Fixed gain in dB.
    ForcedGain(f32),
}

/// Overlay motion and duration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlayTransition {
    /// Cut/fade/slide/wipe setting.
    pub kind: TransitionKind,
    /// Duration on programme timeline.
    pub duration_frames: u64,
}

/// Per-output overlay route; never baked into the mix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlayChannel {
    /// Zero-based channel, 0–9.
    pub index: u8,
    /// Operator label.
    pub name: String,
    /// Assigned source.
    pub source: Option<SourceRef>,
    /// Assignment-specific transform.
    pub geometry: Transform2D,
    /// Premultiplied blend opacity.
    pub opacity: f32,
    /// Lower values draw first.
    pub z_order: i32,
    /// Entry and exit configuration.
    pub transition: OverlayTransition,
    /// Audio policy.
    pub audio: OverlayAudio,
    /// Runtime visibility.
    pub state: OverlayState,
}

/// Base source before per-output overlay compositing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OutputSource {
    /// Clean mix/effect programme.
    Me(MeId),
    /// Direct input.
    Input(InputId),
    /// Multiview composition.
    Multiview,
    /// Opaque black.
    Black,
}

/// Output codec choice; hardware policy is separate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VideoCodec {
    /// H.264, hardware or OpenH264 fallback.
    H264,
    /// HEVC, hardware required.
    Hevc,
}

/// Video output encoding request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoEncoderSettings {
    /// Codec choice.
    pub codec: VideoCodec,
    /// Target bits per second.
    pub bitrate: u64,
    /// Keyframe interval.
    pub gop_frames: u32,
}

/// AAC output configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioEncoderSettings {
    /// Target bits per second.
    pub bitrate: u32,
    /// Encoded channel count.
    pub channels: u8,
}

/// MPEG-TS mux configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TsMuxSettings {
    /// Service identifier.
    pub service_id: u16,
    /// Program-map PID.
    pub pmt_pid: u16,
    /// Video PID.
    pub video_pid: u16,
    /// Audio PID.
    pub audio_pid: u16,
}

/// Output configuration; no media sinks execute in Foundation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OutputSink {
    /// NDI sender.
    Ndi {
        /// Sender name.
        name: String,
        /// Discovery groups.
        groups: String,
        /// SDK video pacing.
        clock_video: bool,
        /// SDK audio pacing.
        clock_audio: bool,
    },
    /// UDP MPEG-TS.
    UdpTs {
        /// Destination address.
        address: String,
        /// Destination port.
        port: u16,
        /// Packet TTL.
        ttl: u8,
        /// Video configuration.
        video: VideoEncoderSettings,
        /// Audio configuration.
        audio: AudioEncoderSettings,
        /// Mux configuration.
        mux: TsMuxSettings,
    },
    /// SRT MPEG-TS.
    Srt {
        /// Endpoint.
        url: String,
        /// Role.
        mode: SrtMode,
        /// Latency in milliseconds.
        latency_ms: u32,
        /// Optional secret.
        passphrase: Option<String>,
        /// Video configuration.
        video: VideoEncoderSettings,
        /// Audio configuration.
        audio: AudioEncoderSettings,
        /// Mux configuration.
        mux: TsMuxSettings,
    },
    /// Fullscreen output window.
    Window {
        /// Target display.
        display_index: u32,
        /// Fullscreen window.
        fullscreen: bool,
    },
    /// Discard composited frames for benchmarks in Phase 5.
    Null,
}

/// Independent output dimensions, rate and colour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputFormat {
    /// Pixels per row.
    pub width: u32,
    /// Pixels per column.
    pub height: u32,
    /// Frames per second.
    pub fps: FrameRate,
    /// Output colour specification.
    pub color_space: ColorSpace,
}

/// Configured output route.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Output {
    /// Identity.
    pub id: OutputId,
    /// Operator label.
    pub name: String,
    /// Active output request.
    pub enabled: bool,
    /// Base source.
    pub source: OutputSource,
    /// Low ten bits select overlays.
    pub overlay_mask: u16,
    /// Independent audio route.
    pub audio_bus: BusId,
    /// Egress format.
    pub format: OutputFormat,
    /// Destination configuration.
    pub sink: OutputSink,
}

/// One of Master/A/B/C/D.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioBus {
    /// Identity.
    pub id: BusId,
    /// Operator label.
    pub name: String,
    /// Number of channels.
    pub channels: u8,
    /// Master fader.
    pub gain_db: f32,
    /// Bus mute.
    pub mute: bool,
}

/// Audio bus configuration; matrix rows reside on inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioConfig {
    /// Master plus A, B, C and D.
    pub buses: Vec<AudioBus>,
}

/// Recording configuration; runtime starts in Phase 8.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recorder {
    /// Identity.
    pub id: RecorderId,
    /// Operator label.
    pub name: String,
    /// Programme or ISO source.
    pub source: OutputSource,
    /// Destination path.
    pub path: PathBuf,
    /// Audio source.
    pub audio_bus: BusId,
    /// Video encoding request.
    pub video: VideoEncoderSettings,
}

/// Programme-wide settings independent of platform APIs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSettings {
    /// Programme format.
    pub format: OutputFormat,
    /// Project audio layout channel count.
    pub audio_channels: u8,
    /// Warning threshold, default 12.
    pub max_hot_inputs: u32,
}

/// Production configuration; file persistence is a later phase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    /// Project label.
    pub name: String,
    /// Programme settings.
    pub settings: ProjectSettings,
    /// Configured sources.
    pub inputs: Vec<Input>,
    /// Data model capacity is 1–4.
    pub mes: Vec<MixEffect>,
    /// Ten fixed overlay routes.
    pub overlays: [OverlayChannel; 10],
    /// Configured egress routes.
    pub outputs: Vec<Output>,
    /// Audio buses.
    pub audio: AudioConfig,
    /// Recording requests.
    pub recorders: Vec<Recorder>,
    /// Separate rundown file path.
    pub rundown: Option<PathBuf>,
}

/// Observed tick delivery for one Foundation sink.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SinkStats {
    /// Sink identity.
    pub id: OutputId,
    /// Ticks offered to this sink.
    pub dispatched: u64,
    /// Oldest ticks evicted on overflow.
    pub dropped: u64,
    /// Approximate current queue depth.
    pub queued: usize,
}

/// Monotonic clock observations published by the control thread.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClockStats {
    /// Most recently dispatched tick.
    pub last_frame: Option<FrameTime>,
    /// Total emitted ticks.
    pub emitted: u64,
    /// Largest observed lateness.
    pub max_lateness_ns: u64,
    /// Lateness of latest tick.
    pub final_lateness_ns: u64,
    /// Ticks at least one frame late.
    pub deadline_misses: u64,
}

/// Authoritative engine snapshot consumed by clients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineState {
    /// Engine-owned project data.
    pub project: Project,
    /// Accepted mutation counter.
    pub revision: u64,
    /// Clock/control lifecycle.
    pub running: bool,
    /// Observed timing.
    pub clock: ClockStats,
    /// Per-sink counters.
    pub sinks: Vec<SinkStats>,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            position: [0.0; 2],
            scale: [1.0; 2],
            rotation: 0.0,
            anchor: [0.5; 2],
        }
    }
}
impl Default for OutputFormat {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: FrameRate::default(),
            color_space: ColorSpace::Bt709,
        }
    }
}
impl Default for Project {
    fn default() -> Self {
        Self {
            name: "Untitled".into(),
            settings: ProjectSettings {
                format: OutputFormat::default(),
                audio_channels: 2,
                max_hot_inputs: 12,
            },
            inputs: Vec::new(),
            mes: vec![MixEffect {
                id: MeId(0),
                name: "M/E 1".into(),
                program: SourceRef::Black,
                preview: SourceRef::Black,
                transition: TransitionSettings {
                    kind: TransitionKind::Cut,
                    duration_frames: 0,
                    easing: Easing::Linear,
                },
                transition_state: 0.0,
                ftb: FadeToBlackState::Off,
            }],
            overlays: std::array::from_fn(|index| OverlayChannel {
                index: index as u8,
                name: format!("Overlay {}", index + 1),
                source: None,
                geometry: Transform2D::default(),
                opacity: 1.0,
                z_order: index as i32,
                transition: OverlayTransition {
                    kind: TransitionKind::Cut,
                    duration_frames: 0,
                },
                audio: OverlayAudio::Ignore,
                state: OverlayState::Off,
            }),
            outputs: vec![Output {
                id: OutputId(0),
                name: "Output 1".into(),
                enabled: false,
                source: OutputSource::Black,
                overlay_mask: 0,
                audio_bus: BusId(0),
                format: OutputFormat::default(),
                sink: OutputSink::Null,
            }],
            audio: AudioConfig {
                buses: ["Master", "A", "B", "C", "D"]
                    .iter()
                    .enumerate()
                    .map(|(i, name)| AudioBus {
                        id: BusId(i as u32),
                        name: (*name).into(),
                        channels: 2,
                        gain_db: 0.0,
                        mute: false,
                    })
                    .collect(),
            },
            recorders: Vec::new(),
            rundown: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn domain_roundtrip_preserves_ids_and_fixed_routes() {
        let project = Project::default();
        let json = serde_json::to_value(&project).unwrap();
        assert_eq!(json["mes"][0]["id"], 0);
        assert_eq!(serde_json::from_value::<Project>(json).unwrap(), project);
        assert_eq!(project.overlays.len(), 10);
        assert_eq!(project.settings.max_hot_inputs, 12);
        assert_eq!(FrameSelection::default(), FrameSelection::Nearest);
        assert!(project.outputs.iter().all(|o| !o.enabled));
    }
}
