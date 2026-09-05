# HasaStudio — Live Production Switcher

**Engineering specification for autonomous implementation.**

Version 1.0 · Target implementer: Codex (GPT-6) · Human reviewer available for architectural sign-off at each phase gate.

---

## 0. How to use this document

This specification is **prescriptive**. Where it names a crate, a file path, a type name, or an algorithm, use exactly that. Where it says *implementer's choice*, decide and record the decision in `docs/decisions/NNNN-title.md` (one ADR per decision).

**Build order is mandatory.** Phases in §13 are sequential. Do not begin a phase until the previous phase's acceptance criteria all pass in CI on all three target platforms. Do not implement features from later phases early, even if they seem trivial — the phase gates exist so that regressions are attributable.

**Never redesign the architecture.** If a phase reveals that §5–§11 are wrong, stop, write an ADR describing the problem and the proposed change, and request human review. Do not silently deviate.

**Do not stub.** A phase is complete when its features work end-to-end, not when the types compile. `todo!()` and `unimplemented!()` must not exist in a merged phase, except behind a feature flag for an explicitly deferred later phase.

---

## 1. What HasaStudio is

HasaStudio is a cross-platform live video production switcher — comparable in role to vMix — used to run a live or semi-automated television-style programme from a mix of file-based, network, and screen-captured sources.

Its distinguishing characteristics versus vMix:

- **No built-in graphics engine.** Graphics are authored externally (SPX Graphics) and enter HasaStudio as ordinary alpha-carrying video sources. HasaStudio's job is compositing and routing, not title rendering.
- **Overlay channels are a routing concern, not a mix concern.** Overlays are composited per-output, so a clean feed and a branded feed are the same mix with different overlay masks.
- **Outputs are independently configured.** Each output selects its own source, its own subset of overlay channels, and its own audio bus.
- **Runs on Windows, macOS, and Linux with a full GUI.** The application is never operated from a command line.

### 1.1 Non-goals for v1

Explicitly out of scope. Do not implement, do not design around:

- Capture cards (DeckLink, Magewell) and UVC/webcam input.
- Title/graphics authoring, character generator, scoreboards.
- Direct SPX Graphics API integration (see §12.1 — the architecture must not preclude it).
- External control surfaces: Companion, MIDI, Stream Deck, GPI, tally output.
- Motion-compensated frame rate conversion.
- Video effects beyond those listed in §7.4.
- Multi-machine / clustered operation.
- Any form of licensing, DRM, or activation.

---

## 2. Target environment

| | |
|---|---|
| Reference GPU | AMD Radeon RX 6800 XT (RDNA2, VCN 3.0, 16 GB) |
| Reference programme format | 1920×1080p50, BT.709 |
| Also supported | 3840×2160p50, and 25/30/60 fps variants |
| Concurrent outputs (v1 target) | 4 |
| Total configured inputs | several dozen (50+ must not break the UI or the project file) |
| Concurrent *decoding* inputs | ~8–12 typical, must degrade gracefully beyond |
| Concurrent NDI receivers | few (1–4 typical) |
| Typical file bitrate | 10–20 Mbps H.264/HEVC |
| Audio | 48 kHz, 32-bit float internally |
| Latency budget | Relaxed. ≤ 250 ms glass-to-glass on NDI output is acceptable; do not trade correctness or image quality for latency. |

### 2.1 Hardware codec implications

The reference GPU is **AMD**. Do not write NVENC-shaped code and abstract it later.

| Platform | Decode | Encode |
|---|---|---|
| Windows | D3D11VA | AMF (H.264, HEVC) |
| Linux | VAAPI | VAAPI (H.264, HEVC) |
| macOS | VideoToolbox | VideoToolbox (H.264, HEVC) |

RDNA2 has **no AV1 encoder**. AV1 decode is available and should be used when present. Decode always has a software fallback, via FFmpeg's own LGPL decoders. Encode software fallback is OpenH264 for H.264 only. There is no software HEVC encoder; HEVC output requires hardware encode and its absence is a clear GUI error, never a silent failure or a silent downgrade.

This is already flagged for commercial review as §16 item 2. Do not resolve it further.

---

## 3. Technology stack (locked)

| Concern | Choice | Notes |
|---|---|---|
| Language | Rust, stable toolchain, edition 2021 | MSRV pinned in `rust-toolchain.toml`, bumped only via ADR |
| GPU / compositing | `wgpu` | D3D12 on Windows, Vulkan on Linux, Metal on macOS |
| Codecs, demux, mux | FFmpeg 7.x via `ffmpeg-next` | LGPL build; see §3.1 |
| NDI | NDI SDK 6 (input and output) | Dynamically loaded at runtime; see §3.1 |
| GUI | `egui` + `eframe`, rendered on the compositor's own `wgpu` device | Phase 0 uses an empty eframe window; shared-device rendering begins in Phase 1. Video preview textures are sampled directly; no CPU readback for preview |
| Audio DSP | Hand-written in `rezie-audio`; `rubato` for resampling | No external audio framework |
| Audio device I/O | `cpal` | Monitoring output and future device input only — not on the programme path |
| Serialisation | `serde`; YAML via `serde_yaml` for project and rundown files | See §10 |
| Async runtime | `tokio` for I/O and control plane only | The media path uses dedicated OS threads, never the async runtime |
| Logging | `tracing` + `tracing-subscriber` | Structured, with a rolling file appender and an in-GUI log pane |
| Errors | `thiserror` in libraries, `anyhow` at the binary boundary | No `unwrap()` on any path reachable from user action |

### 3.1 Licensing and linking constraints

The project is closed-source-capable. This constrains linking:

- FFmpeg must be **LGPL**, built without `--enable-gpl` and without `--enable-nonfree`, **dynamically linked**. This means no `libx264`/`libx265` linked in-tree. For software H.264/HEVC encoding, use OpenH264 (BSD, dynamically loaded) for H.264 and accept that software HEVC encoding is unavailable — hardware HEVC is a hard requirement for HEVC output. Record this in an ADR.
- The NDI SDK must be **loaded at runtime** via `libloading`, never linked at build time. If the runtime is absent, all NDI features are disabled with a clear GUI message and a link to the download; the application must still start and function fully otherwise.
- SRT (`libsrt`) is MPL-2.0 and may be dynamically linked.
- CEF (§12.1) is BSD and runs as a separate process, so it imposes no constraint on the main binary.

The NDI SDK is never fetched: it requires user licence acceptance and installation. `xtask fetch-deps` uses a per-dependency `from_phase` manifest and never fetches before the consuming phase. CEF must never be fetched before Phase 10. Phase 0 tests hash-verified fetching against a real Phase 0 dependency.

Vendor no source. Build-linked native dependencies are resolved at build time by `build.rs` scripts using `pkg-config` on Linux/macOS and a pinned prebuilt bundle on Windows, downloaded and hash-verified by `xtask fetch-deps`.

---

## 4. Repository layout

```
rezie/
├── Cargo.toml                  # workspace
├── rust-toolchain.toml
├── xtask/                      # cargo xtask: fetch-deps, package, dist, ci
├── docs/
│   ├── decisions/              # ADRs
│   ├── schema/                 # JSON Schema for project + rundown files
│   └── user/                   # end-user documentation, written as phases land
├── crates/
│   ├── rezie-core/             # domain model, project state, command bus, clock, scheduler
│   ├── rezie-gpu/              # wgpu device, frame pool, shaders, compositor graph
│   ├── rezie-media/            # FFmpeg: demux, decode, encode, mux; source & sink traits
│   ├── rezie-audio/            # mixer, buses, per-channel DSP, resampling, metering
│   ├── rezie-ndi/              # NDI SDK loader, receiver, sender, discovery
│   ├── rezie-net/              # SRT/RTMP/RTSP ingest; MPEG-TS/UDP/SRT egress
│   ├── rezie-capture/          # screen and window capture, per-platform
│   ├── rezie-html/             # HTML source client + IPC protocol (phase 10)
│   ├── rezie-rundown/          # rundown model, YAML (de)serialisation, scheduler
│   ├── rezie-engine/           # assembles the above into a headless runnable engine
│   ├── rezie-api/              # control API: command/event types, in-process + WebSocket transports
│   └── rezie-app/              # egui GUI; the shipped binary
└── tests/
    ├── integration/            # engine-level tests driving rezie-api
    ├── assets/                 # small generated test media, committed
    └── golden/                 # reference frames for compositor regression tests
```

`rezie-app` depends on `rezie-engine` and `rezie-api`. **No other crate may depend on `rezie-app`.** The engine must be fully exercisable without any GUI code linked in — this is what makes it testable.

---

## 5. Architecture

### 5.1 Engine / GUI separation

The engine is headless and owns all state. The GUI is a client that sends commands and receives events. In the shipped application both live in one process and communicate over in-memory channels; the same `rezie-api` command and event types are also exposed over a local WebSocket, used by the integration test harness.

This is not a user-facing feature and must not be advertised in the GUI. It exists so that:

1. Every feature is testable without driving a GUI.
2. The GUI cannot hold state that the engine does not have — which is what makes project save/load and rundown automation correct by construction.

```
┌──────────────┐   Command    ┌────────────────────────────┐
│  rezie-app   │─────────────▶│        rezie-engine        │
│    (egui)    │◀─────────────│                            │
└──────────────┘   Event      │  ┌──────────────────────┐  │
                              │  │  state (authoritative)│  │
┌──────────────┐   Command    │  └──────────────────────┘  │
│ test harness │─────────────▶│  ┌──────────────────────┐  │
│  (WebSocket) │◀─────────────│  │  media threads       │  │
└──────────────┘   Event      │  └──────────────────────┘  │
                              └────────────────────────────┘
```

**Commands** are total descriptions of intent (`SetProgramSource { me: MeId, source: SourceRef }`), never deltas. **Events** describe what actually happened, including rejections. The GUI renders engine state; it must never optimistically mutate its own copy.

### 5.2 Threading model

| Thread | Count | Responsibility |
|---|---|---|
| Control | 1 | Owns engine state. Processes commands, emits events. Never blocks on I/O or GPU. |
| Clock/composite | 1 | Wakes each programme frame. Gathers frames, runs the compositor, dispatches to sinks. |
| Decoder | 1 per active decoding input | Demux + decode + upload. Writes into that input's frame ring. |
| Encoder/sink | 1 per output | Consumes composited frames, encodes, muxes, transmits. |
| Audio | 1 | Mixes all buses at block rate, feeds sinks and meters. |
| GUI | 1 (main) | egui. Reads a lock-free snapshot of engine state. |
| Async runtime | pool | Discovery, network control, file dialogs, HTML IPC. Never on the media path. |

State that crosses threads: use `arc-swap` for read-mostly snapshots (engine state → GUI, parameters → compositor) and bounded `crossbeam` channels for work. **No `Mutex` on the composite thread.** If the composite thread ever blocks, the whole programme output stutters; treat any lock acquisition there as a bug.

### 5.3 The clock

A single monotonic master clock defines the programme timeline.

- The composite thread targets `1/fps` intervals derived from `Instant`, with drift correction against an absolute frame counter — never accumulate per-frame sleep error.
- At 50 fps and 48 kHz, one frame is exactly 960 audio samples. Choose supported programme rates so this stays exact where possible; where it does not (e.g. 59.94), maintain a fractional accumulator and vary the block size between 800 and 801 samples.
- Every produced frame carries `FrameTime { index: u64, pts: Duration }` on the programme timeline. All sinks and all recordings share this timeline, so an ISO recording and the programme recording are frame-aligned by construction.
- Output sinks may run behind. Each sink has a bounded queue; on overflow it **drops the oldest frame and increments a dropped-frame counter surfaced in the GUI**. A slow sink must never stall the composite thread.

### 5.4 Resource policy (mandatory)

With several dozen configured inputs, decoding everything is not viable. Each input has a runtime state:

| State | Meaning | Cost |
|---|---|---|
| `Cold` | Configured, not open | Metadata only |
| `Warm` | Container open, decoder initialised, first frames decoded, paused | ~1 GOP buffered |
| `Hot` | Actively decoding into its frame ring | Full |

Promotion rules, evaluated on the control thread whenever routing changes, with Hot taking precedence over Warm:

- `Hot` — on any M/E programme bus, on any M/E preview bus, assigned to an **enabled** overlay channel, selected as any output's source, or explicitly playing.
- `Warm` — assigned to a **disabled** overlay channel, within the rundown lookahead window, or explicitly warmed by the operator.
- `Cold` — otherwise.

Preview is Hot. The operator is looking at it.

Live sources also have a Warm state:

- `Cold` — not connected.
- `Warm` — connected for multiview only; NDI uses `NDIlib_recv_bandwidth_lowest`.
- `Hot` — connected at full bandwidth.

Multiview thumbnails are rendered from whatever the input's current state provides. A `Cold` input shows its poster frame or a placeholder; it does not force promotion.

Promotion must be pre-emptive: when the rundown scheduler knows an item is next, it warms it early (§10.4). A visible black flash on take because the decoder was cold is a defect.

`max_hot_inputs` is a project setting (default 12). Exceeding it logs a warning and shows a GUI indicator, but is not blocked — the operator may know their hardware better than the default does.

---

## 6. Domain model

Defined in `rezie-core`. All IDs are newtype-wrapped `u32` with `serde` transparent representation.

```
Project
├── settings: ProjectSettings         # programme format, colour space, audio layout
├── inputs: Vec<Input>
├── mes: Vec<MixEffect>               # 1..=4
├── overlays: [OverlayChannel; 10]
├── outputs: Vec<Output>              # 1..=8 configured, 4 concurrent target
├── audio: AudioConfig                # buses, routing matrix
├── recorders: Vec<Recorder>
└── rundown: Option<PathBuf>          # rundown lives in its own file (§10)
```

### 6.1 Input

```
Input {
  id, name, kind: InputKind,
  playback: PlaybackSettings,          # file-like sources only
  process: InputProcessing,            # applied once, shared by all consumers
  audio: InputAudio,
  state: InputState                    # runtime, not persisted
}
```

`InputKind` (v1):

- `File { path, start_at, end_at }` — video and/or audio container
- `Image { path }` — still, alpha preserved (PNG, TGA, TIFF, WebP)
- `ImageSequence { pattern, fps }`
- `Color { rgba }`
- `Ndi { source_name, bandwidth }`
- `Srt { url, mode: Caller|Listener, latency_ms, passphrase }`
- `Rtmp { url }` — pull only in v1
- `Rtsp { url, transport: Tcp|Udp }`
- `ScreenCapture { target: Display(id) | Window(id) | Region(rect), cursor: bool }`
- `Html { url, width, height, fps }` — phase 10
- `AudioFile { path }`
- `MeOutput { me: MeId }` — an M/E's programme output usable as a source in another M/E; the routing graph must be validated as acyclic and a cycle rejected with a clear error

### 6.2 Mix/effect

```
MixEffect {
  id, name,
  program: SourceRef,
  preview: SourceRef,
  transition: TransitionSettings,      # type, duration, easing
  transition_state: f32,               # 0.0 = program, 1.0 = preview; T-bar position
  ftb: FadeToBlackState
}
```

Ship with 1 M/E in phase 2. The model, compositor, and GUI must support up to 4 from the start; enable 2–4 in phase 11 once performance is measured. On the reference GPU an additional M/E costs one extra composite pass — cheap. The real cost is that its sources must be `Hot`, so the resource policy governs the practical limit, not the GPU.

### 6.3 Overlay channel

Ten channels, fixed. Each:

```
OverlayChannel {
  index: 0..10, name,
  source: Option<SourceRef>,
  geometry: Transform2D,               # independent of the input's own transform
  opacity: f32,
  z_order: i32,                        # default = index; lower draws first
  transition: OverlayTransition,       # cut | fade | slide(dir) | wipe(dir), with duration
  audio: OverlayAudio,                 # follow | ignore | forced gain
  state: OverlayState                  # Off | TransitioningIn(t) | On | TransitioningOut(t)
}
```

Overlays are composited **per-output**, after the output has selected its base source. They are not baked into M/E programme output. An M/E's programme is always a clean feed.

Overlay commands: `OverlayOn`, `OverlayOff`, `OverlayToggle`, `OverlayOffAll`, each optionally overriding the transition.

### 6.4 Output

```
Output {
  id, name, enabled,
  source: OutputSource,                # Me(id) | Input(id) | Multiview | Black
  overlay_mask: u16,                   # bitmask over the 10 overlay channels
  audio_bus: BusId,
  format: OutputFormat,                # resolution, fps, may differ from programme
  sink: OutputSink
}
```

`OutputSink`:

- `Ndi { name, groups, clock_video, clock_audio }`
- `UdpTs { address, port, ttl, video: VideoEncoderSettings, audio: AudioEncoderSettings, mux: TsMuxSettings }`
- `Srt { url, mode, latency_ms, passphrase, ... same encoder settings }`
- `Window { display_index, fullscreen }` — a fullscreen output window on a secondary display
- `Null` — composites but discards; for benchmarking

Format conversion between programme format and output format happens in the output stage: Lanczos downscale on GPU for resolution, frame drop/repeat for integer fps ratios (50→25), and rejection with a clear error for non-integer ratios (50→30) unless the operator explicitly opts into blending.

---

## 7. Video pipeline

### 7.1 Frame representation

All frames on the GPU are `Rgba16Float` in linear light, BT.709 primaries, premultiplied alpha. Rationale: overlay compositing with transitions requires linear-light blending to avoid dark fringing; 16-bit float removes banding from repeated blends; premultiplied alpha makes the blend operator associative, which the overlay stack needs.

Conversion to and from this working space happens exactly twice: on ingest (after decode) and on egress (before encode). Nowhere else.

`rezie-gpu` owns a `FramePool` of reusable GPU textures, keyed by dimensions and format, with reference counting. Allocation on the composite thread is forbidden; the pool pre-allocates and grows only from the control thread.

### 7.2 Ingest chain (per input, once per frame, result shared by all consumers)

```
demux → decode (HW, fallback SW) → upload to GPU
  → deinterlace (if flagged interlaced)
  → colour convert to working space
  → crop
  → colour correction
  → chroma key
  → [cached as the input's "processed texture" for this frame]
```

**Deinterlacing.** Detect interlacing from container/stream metadata, allow manual override per input (`Auto | Progressive | TopFieldFirst | BottomFieldFirst`). Implement a bwdif-equivalent motion-adaptive deinterlacer as a compute shader — bwdif produces visibly better results than yadif on the diagonal detail that yadif's spatial interpolation smears, and the cost difference is negligible on the reference GPU. Deinterlace to full frame rate (each field becomes a frame) by default, with a per-input option for field-to-frame (half rate). If the shader implementation is deferred, use FFmpeg's `bwdif` CPU filter as an interim, but record it as debt.

### 7.3 Frame rate conversion (this is the part that goes wrong quietly)

The programme clock is authoritative. Inputs are never resampled at decode time. Each input's decoded frames go into a ring buffer tagged with a presentation time mapped onto the programme timeline; the composite thread selects from that ring each tick.

Selection policy, per input:

- **`Nearest` (default).** Choose the frame whose display interval contains the programme frame's midpoint. For 25→50 this is exact 2:2 repetition. For 24→50 or 30→50 it produces mild, uniform judder — which is correct and is what broadcast chains do. No ghosting, no softness.
- **`Blend` (opt-in per input).** Linearly cross-dissolve the two bracketing frames weighted by temporal phase. Smoother on slow pans and graphics motion, ghosts on fast motion. Offer it, do not default to it.
- Live sources (NDI, SRT, RTMP, RTSP, screen capture) always use `Nearest` against a small jitter buffer (default 3 frames, configurable per input). `Blend` is not offered for them.
- 23.976/29.97/59.94 content in a 50 or 25 fps programme: `Nearest` with the fractional accumulator. Do not attempt pulldown detection in v1.

Audio drift is handled separately in §8.4.

### 7.4 Per-input processing

- **Transform.** Position, scale (uniform and non-uniform), rotation, anchor point. Bilinear when scaling up, Lanczos-3 when scaling down by more than 1.5×, mipmapped otherwise.
- **Crop.** Source-relative rectangle, applied before transform.
- **Chroma key.** YUV-space distance keyer with: key colour picker, similarity, smoothness, spill suppression, edge choke/expand, and a preview mode showing the alpha matte as greyscale. This must be good enough for a real green screen, not a demo. Implement in a single compute pass.
- **Colour correction.** Lift/gamma/gain, saturation, hue rotation, contrast. No LUTs in v1.
- **Alpha.** Sources with native alpha (PNG, image sequences, NDI, HTML, ProRes 4444, VP9/WebM with alpha) must preserve it end to end. This is not optional — it is the mechanism by which SPX graphics work.

### 7.5 Composite stages

```
Stage 1 — Input processing
    for each Hot input: produce processed texture (§7.2, §7.4)

Stage 2 — Mix
    for each M/E:
      program_tex  = processed(program_source)
      preview_tex  = processed(preview_source)
      me_out       = transition_shader(program_tex, preview_tex, transition_state, type)
      apply FTB
    M/E outputs are clean — no overlays.

Stage 3 — Output composite (per enabled output)
    base = select(output.source)                    # M/E out, input, multiview, black
    for each overlay channel in z_order:
      if channel enabled in output.overlay_mask and channel.state != Off:
        base = blend(base, overlay_tex, channel.opacity * transition_progress, channel.geometry)
    convert working space → output colour space
    resize to output format
    → sink

Stage 4 — Multiview
    render tiles at reduced resolution and reduced rate (default 12.5 fps)
```

Stages 1 and 2 run once per programme frame regardless of output count. Only stage 3 scales with outputs. This is why 4 outputs is cheap and why overlay-per-output is architecturally free.

Transition types for v1: cut, fade (dissolve), wipe (L/R/U/D, with softness), slide (L/R/U/D), stinger (an alpha-carrying input played as a transition, with a configurable cut point in frames). Each has a duration in frames and an easing curve (`linear | ease_in_out | custom_cubic`).

### 7.6 Golden-frame testing

The compositor is the component where regressions are least visible and most damaging. `tests/golden/` holds reference PNGs. Every compositor test renders a deterministic scene from committed test assets and compares against its reference with a perceptual metric (Δ, not bit-exact — GPU vendors differ). Threshold: mean ΔE < 1.0, max ΔE < 3.0. Failures write the actual and difference images to `target/golden-failures/` for review.

Generate the test assets in `xtask gen-assets` from code — colour bars, a moving gradient, an alpha logo, a green-screen plate — so nothing large is committed.

---

## 8. Audio

### 8.1 Buses

vMix-style. `Master` plus `A`, `B`, `C`, `D`. Each bus is a stereo (or, per §8.5, multichannel) mix with its own master fader, mute, and meter.

Every input has a **routing matrix row**: for each bus, on/off plus an optional per-bus gain offset. This is the mechanism for mix-minus — a talent feed bus is built by enabling every source on bus B except the talent's own return.

Bus assignment is independent of video routing. An input can be audible on Master while invisible.

### 8.2 Per-input audio processing

Order is fixed:

```
decode → resample to 48 kHz → delay → gain → EQ → compressor → pan → bus sends
```

- **Delay.** 0–1000 ms, for lip-sync correction.
- **Gain.** −∞ to +12 dB, with a fader curve matching vMix's for operator familiarity.
- **EQ.** Three bands: low shelf, peaking mid (with Q), high shelf. Biquad, coefficients recomputed on the control thread and swapped atomically.
- **Compressor.** Threshold, ratio, attack, release, makeup gain. Feed-forward, peak-sensing, with soft knee. One good compressor, not a rack.
- **Pan.** Constant-power.
- **Mute / solo.** Solo is exclusive within a bus and does not affect other buses.
- **Audio Follow Video (AFV).** Per input per bus. When AFV is on for a bus, the input's contribution to that bus is gated by whether the input is visible on the M/E feeding an output that uses that bus, and follows the video transition envelope — a dissolve fades the audio across the same duration.

### 8.3 Metering

Per input, per bus: peak and RMS, with a 1.5 s peak hold and a clip indicator that latches until clicked. Meters run on the audio thread and publish through an `arc-swap` snapshot; the GUI never pulls samples.

### 8.4 Clock and drift

Audio is generated in blocks aligned to programme frames (§5.3). Each source's audio passes through an **asynchronous resampler** (`rubato`, sinc interpolation) whose ratio is continuously adjusted by a slow control loop tracking the fill level of that source's audio buffer against the master clock.

- Target buffer depth: 3 frames of audio.
- Correction is bounded to ±0.5% and rate-limited so it is inaudible.
- If a buffer underruns, insert silence and log; do not stall.
- If it persistently overruns beyond the correction range (a genuinely wrong-rate source), drop a block, log a warning, and surface it in the GUI.

Never correct drift by dropping or duplicating samples without resampling — the click is audible.

### 8.5 Channel layout

Internally the mixer is N-channel. Default project layout is stereo. Support up to 8 channels per bus, and up to 16 channels on NDI outputs (NDI carries them natively). Encoded outputs (UDP/SRT) are stereo or 5.1 via AAC. Downmix matrices for N→2 must be present and standard (ITU-R BS.775 coefficients).

### 8.6 Monitoring

One local audio device output, selectable, fed from a selectable bus, with its own independent fader. This is headphone monitoring for the operator and must never be on the programme path.

---

## 9. Outputs and recording

### 9.1 NDI output

Full alpha support. Configurable source name and groups. Video and audio clocked to the programme clock. Send frames as `UYVY` (or `UYVA` when the output carries alpha) — converted on GPU, read back once. Use async send with the SDK's own buffering; do not block the composite thread.

### 9.2 UDP MPEG-TS output

- Mux: MPEG-TS, configurable PMT/PID assignment, PCR interval, and TS packet-per-datagram count (default 7, giving 1316-byte payloads).
- Video: H.264 or HEVC, hardware-encoded, CBR or VBR with configurable bitrate, GOP length, B-frames, and profile/level.
- Audio: AAC-LC or MPEG-1 Layer II (the latter for legacy broadcast interoperability).
- Destination: unicast or multicast with configurable TTL and source interface selection.
- Also implement SRT output using the same encoder and mux stage — it is a different transport under an identical pipeline, so the incremental cost is small and it is far more useful than raw UDP over anything but a LAN.

### 9.3 Recording

- **Programme recorder.** Records any output's composited frames (i.e. it can record the branded feed or the clean feed) to MP4, MOV, or MKV. Hardware-encoded H.264 or HEVC. Configurable bitrate and container.
- **ISO recorders.** Record individual inputs, *pre-processing* by default (the raw decoded source) with an option for post-processing. Configurable per input, with a hard cap surfaced in the UI — on the reference hardware, budget 4 concurrent 1080p50 ISO recorders alongside 4 outputs and validate this in phase 8's benchmark.
- All recordings share the programme timeline, so they are frame-aligned. Write the programme frame index into a sidecar `.rzt` timecode file alongside each recording for post-production alignment.
- **Robustness.** Use fragmented MP4 or MKV so a crash or power loss leaves a playable file. Never write a container that requires clean finalisation. Disk write happens on the sink's own thread with a bounded queue; on sustained overflow, stop the recording, log loudly, and show a persistent GUI error — do not silently produce a corrupt file.
- Filename templating: `{project}_{output}_{date}_{time}_{n}` with automatic increment.

### 9.4 Multiview

A composited grid showing all `Hot` and `Warm` inputs plus programme and preview, with per-tile labels, tally borders (red = on programme, green = on preview, yellow = on an enabled overlay), and audio meters. Renders at reduced resolution and rate (§7.5). Available as an output sink so it can be sent to a second display or to NDI.

---

## 10. Rundown and playlist

### 10.1 Requirements

The rundown drives semi-automated programme playout. It must be:

- **Text-editable outside the application.** Scenarios are prepared in advance, sometimes by people who do not have HasaStudio open.
- **Human-readable and diffable.** It will live in version control.
- **Schema-validated on load,** with errors that name the line and column and say what was expected.

Format: **YAML**, extension `.rezie-rundown.yaml`. A JSON Schema in `docs/schema/rundown.schema.json` is the normative definition; `serde` types must be generated from or validated against it in CI.

### 10.2 Timing model

Each item has a `timing` mode:

- `manual` — waits for the operator to take it.
- `follow` — starts when the previous item completes (a file playing to its end, or a fixed duration elapsing).
- `at: "14:30:00"` — hard clock time. The scheduler takes it at that wall-clock time regardless of what is currently playing, unless `guard: soft` is set, in which case it waits for the current item to finish and logs the overrun.
- `after: 00:05:00` — a fixed offset from rundown start.

The scheduler computes a projected timeline for the whole rundown and displays, per item, its projected start and whether it will collide with a later `at` item. **Collision detection is a core feature, not a nicety** — the operator needs to see at 13:00 that the block will overrun the 14:30 hard start.

### 10.3 Item and event types

An item is a primary action plus zero or more secondary events with offsets relative to the item's start (negative offsets allowed, relative to its end).

Primary actions: `take` (to an M/E, with transition override), `preview`, `play`, `pause`, `stop`.

Secondary events: `overlay_on`, `overlay_off`, `overlay_toggle`, `audio` (bus/gain/mute changes), `output` (change an output's source or overlay mask), `record_start`, `record_stop`, `graphic` (reserved for §12.1; in v1 it is a no-op that logs), `macro` (a named sequence defined elsewhere in the file), `wait`.

### 10.4 Lookahead and pre-roll

The scheduler maintains a lookahead window (default 30 s). Any input referenced by an item entering the window is promoted to `Warm` (§5.4) and, if it is a file, seeked to its in-point. An item may set `preroll: 3s` to force earlier warming — necessary for slow network sources.

### 10.5 Example

```yaml
version: 1
name: "Evening block — 2026-09-05"
defaults:
  me: 1
  transition: { type: fade, duration: 25 }

macros:
  lower_third:
    - { action: overlay_on,  channel: 2 }
    - { action: overlay_off, channel: 2, offset: "+8s" }

items:
  - id: opener
    label: "Opening titles"
    timing: { mode: at, at: "19:00:00", guard: hard }
    action: { take: { input: "titles_open" }, transition: { type: cut } }
    secondary:
      - { action: overlay_on, channel: 1, offset: "+2s" }        # bug/logo
      - { action: record_start, recorder: programme, offset: "0s" }

  - id: host_intro
    label: "Host intro"
    timing: { mode: follow }
    action: { take: { input: "cam_host" } }
    secondary:
      - { macro: lower_third, offset: "+3s" }

  - id: package_a
    label: "Package: mountain bike report"
    timing: { mode: follow }
    preroll: "5s"
    action: { take: { input: "vt_mtb_report" }, transition: { type: fade, duration: 12 } }
    secondary:
      - { action: overlay_off, channel: 1, offset: "0s" }         # clean during VT
      - { action: overlay_on,  channel: 1, offset: "-2s" }        # 2s before it ends

  - id: results
    label: "Results板 — hard start"
    timing: { mode: at, at: "19:14:30", guard: soft }
    action: { take: { input: "results_page" } }
    secondary:
      - { action: audio, bus: master, input: "music_bed", gain: -18 }
```

Note `guard: soft` on the last item: it will wait for the package to finish and log the overrun rather than cutting mid-sentence.

### 10.6 Playlists

A playlist is a degenerate rundown: an ordered list of file inputs with `timing: follow` and an optional loop flag. Implement it as a rundown preset in the UI rather than a second subsystem.

---

## 11. GUI

`egui`, immediate mode, rendered on the compositor's `wgpu` device so video textures are sampled directly.

### 11.1 Layout

```
┌────────────────────────────────────────────────────────────┐
│ menu · project · engine status (fps, dropped, CPU/GPU/VRAM) │
├──────────────────────────────┬─────────────────────────────┤
│                              │                             │
│         PREVIEW              │         PROGRAM             │
│                              │                             │
├──────────────────────────────┴─────────────────────────────┤
│ transition bar: [CUT] [AUTO] [FTB] type▾ dur▾  ══T-bar══   │
├─────────────────────────────────────────────────────────────┤
│ input strip — scrollable, tally-bordered thumbnails         │
│ [1 cam][2 vt][3 gfx][4 ndi][5 …]                            │
├─────────────────────────────────────────────────────────────┤
│ overlay strip — 10 buttons, lit when on, source name below  │
├─────────────────────────────────────────────────────────────┤
│ tabs: Rundown │ Audio Mixer │ Outputs │ Recording │ Log      │
└─────────────────────────────────────────────────────────────┘
```

Docking/undocking of the tab area onto a second monitor is required — operators run this on two screens.

### 11.2 Non-negotiable interaction rules

- **Every destructive or on-air action is one deliberate click.** Take, cut, and overlay buttons act immediately with no confirmation dialog. Delete-input and clear-rundown require confirmation. Never put a modal in front of an on-air control.
- **Tally is everywhere.** Any thumbnail, list row, or output row showing a source that is live carries a red border. Preview is green. On an enabled overlay is yellow. This is consistent across every surface in the application.
- **The GUI never lies.** If the engine rejected a command, the GUI shows the pre-command state. No optimistic updates.
- **Keyboard.** Number keys take inputs to preview, `Ctrl+number` takes to programme, space is cut, Enter is auto-transition, `F1`–`F10` toggle overlays, `Esc` is FTB. All rebindable, persisted per user, not per project.
- **Performance.** The GUI must not drop below 30 fps with 50 configured inputs and 4 active outputs. Multiview thumbnails at 12.5 fps are fine; the UI's own responsiveness is not.

### 11.3 Input configuration dialog

Tabs: General (name, source, playback), Video (transform, crop, colour, chroma key — with a live split-screen preview showing before/after and a matte view), Audio (routing matrix, gain, delay, EQ, compressor), Advanced (decoder selection, jitter buffer, frame-rate policy).

Every parameter change is a command to the engine and takes effect on the next frame. No apply button.

---

## 12. Forward compatibility

Do not implement these. Do ensure the architecture does not preclude them, and note in ADRs where a choice was made for this reason.

### 12.1 SPX Graphics integration

Planned. Graphics currently arrive as HTML inputs (phase 10) or NDI. Later, HasaStudio should control SPX directly: trigger, update, and stop graphics items from rundown secondary events.

Consequences for v1 design:
- The rundown's `graphic` secondary event type exists now as a logged no-op with a `target` and free-form `payload` field. Its schema must not need to change.
- The HTML source (phase 10) must expose a JavaScript message channel (engine → page) even though v1 sends nothing over it.

### 12.2 External control

REST/WebSocket control, Companion, MIDI, and tally output are deferred. The `rezie-api` command/event boundary (§5.1) is what makes them cheap later — every operator action must be expressible as a command, with no GUI-only paths.

### 12.3 Capture hardware

DeckLink and UVC. The `VideoSource` trait in `rezie-media` must be implementable by a device driver without changes to the trait. Do not bake file-or-network assumptions into it (in particular: no seek, no duration, no pause on the trait — those belong on a `SeekableSource` sub-trait).

---

## 13. Phases

Each phase ends with: all acceptance criteria passing in CI on Windows, macOS, and Linux; no `todo!()`; ADRs written for any implementer's-choice decisions; user documentation for the features added.

---

### Phase 0 — Foundation

Workspace, `xtask`, CI on three platforms, dependency fetching, `rezie-core` domain types, the clock, the command/event API, the in-process and WebSocket transports, and a headless engine that starts, ticks a clock, and produces timed frame ticks through the sink dispatch path.

**Accepts when:** `cargo test --workspace` passes on all three platforms · the headless engine runs at 50 fps for 10 minutes with measured monotonic-clock drift strictly under one frame (20 ms); this drift bound is normative · the WebSocket harness can connect, send a command, and receive an event · `xtask dist` produces a runnable (empty) application bundle on each platform.

---

### Phase 1 — Media foundation and first picture

`rezie-gpu` device, frame pool, working colour space. `rezie-media` file decode (H.264, HEVC, VP9, AV1; MP4, MOV, MKV, TS), image sources, colour source. Hardware decode with software fallback. A single NDI output. A minimal GUI: one preview pane, an input list, add/remove input.

**Accepts when:** a 1080p50 H.264 file decodes and appears in the preview · the same file appears on an NDI output receivable by NDI Studio Monitor · hardware decode is active on all three platforms, verified by an explicit test that asserts the decoder name · killing hardware decode support (via env override) transparently falls back to software · a PNG with alpha renders with correct transparency over a colour source · the frame pool allocates zero times during a 5-minute steady-state run, asserted by a counter.

---

### Phase 2 — The mixer

One M/E. Programme and preview buses. Cut, auto-transition, T-bar. Transition types: cut, fade, wipe, slide. FTB. Programme and preview panes in the GUI. Tally.

**Accepts when:** golden-frame tests pass for each transition type at 0%, 25%, 50%, 75%, 100% · a T-bar drag produces a monotonic transition with no frame where the mix ratio goes backwards · an auto-transition of N frames takes exactly N frames · cut during an auto-transition resolves immediately and correctly · tally state is correct in the GUI for every routing combination, tested through the API.

---

### Phase 3 — Input processing and playback

Transform, crop, colour correction, chroma key. Playback: play/pause/stop, in/out points, loop, autoplay, playback rate. Frame rate conversion policy (§7.3). Deinterlacing. Input configuration dialog.

**Accepts when:** golden-frame tests for the chroma keyer against a committed green-screen plate, including spill suppression · a 25 fps source in a 50 fps programme produces exact 2:2 with no timing drift over 10 minutes · a 30 fps source in a 50 fps programme produces the documented judder pattern and no dropped or duplicated frames beyond it · an interlaced 1080i25 source deinterlaces to 1080p50 and passes a golden-frame comparison · seeking to an in-point and playing produces the correct first frame, verified by hash.

---

### Phase 4 — Overlays

Ten overlay channels. Per-channel source, geometry, opacity, z-order, transitions. Overlay strip in the GUI with keyboard bindings.

**Accepts when:** all ten channels can be active simultaneously with correct z-ordering, golden-frame verified · an alpha PNG on an overlay channel over a video input composites with no dark fringing (this catches non-linear-light blending — make it an explicit test) · overlay fade in and out are symmetrical and complete in exactly the configured duration · `OverlayOffAll` clears all channels within one frame · warm/cold promotion works for overlay sources, verified by asserting no black frame on enable.

---

### Phase 5 — Outputs

Multiple outputs with independent source, overlay mask, and format. NDI, UDP MPEG-TS, SRT, fullscreen window, null. Format conversion. Output configuration UI.

**Accepts when:** four simultaneous 1080p50 outputs run for 30 minutes with zero dropped frames on the reference hardware · one output shows overlays and another, sharing the same M/E, shows none — verified frame-by-frame on both NDI receivers · a UDP MPEG-TS stream plays correctly in VLC and ffplay and analyses clean in `tsduck` · an SRT stream survives 2% simulated packet loss · a deliberately stalled sink (SIGSTOP on a receiver) does not affect the other three outputs' frame timing.

---

### Phase 6 — Audio

Full mixer: buses, routing matrix, per-input DSP chain, AFV, metering, monitoring, drift correction, multichannel, downmix. Audio mixer tab.

**Accepts when:** a 1 kHz sine through the full chain at unity gain measures within 0.1 dB at the bus output · THD+N below −90 dB through gain, EQ (flat), and pan · a source with a 0.1% clock error runs for one hour with no dropout, no click, and buffer depth held within ±1 frame of target · AFV audio follows a 25-frame video dissolve with a matching envelope, verified by sample analysis · mix-minus configured on bus B correctly excludes exactly one source · meters match a reference measurement within 0.5 dB.

---

### Phase 7 — Network and capture sources

NDI receive with discovery and bandwidth modes. SRT, RTMP, RTSP ingest. Screen and window capture on all three platforms. Jitter buffering and reconnection.

**Accepts when:** NDI discovery finds sources within 2 seconds and the list stays accurate as sources appear and disappear · an NDI source with alpha preserves it · a network source that disconnects reconnects automatically within 5 seconds without operator action and without disturbing the programme · a source with 50 ms of jitter plays without visible stutter at the default buffer setting · screen capture works on Windows (WGC), macOS (ScreenCaptureKit, including permission handling), and Linux (PipeWire portal, with X11 fallback) · a `Warm` NDI source is confirmed to consume `lowest` bandwidth, asserted via SDK query.

---

### Phase 8 — Recording

Programme and ISO recorders. Fragmented containers. Timecode sidecars. Recording UI with disk space monitoring.

**Accepts when:** a 30-minute programme recording plays correctly and its duration is within one frame of expected · a recording is playable after `kill -9` mid-record · a programme recording and two ISO recordings, started at different times, align frame-exactly using their sidecars · four concurrent 1080p50 ISO recorders plus four outputs sustain on the reference hardware for 30 minutes, or the documented cap is lowered and justified with measurements · disk exhaustion produces a clean stop and a persistent error, never a corrupt file.

---

### Phase 9 — Rundown

Rundown model, YAML schema and round-trip, scheduler with all timing modes, secondary events, macros, lookahead and pre-roll, collision detection. Rundown tab with projected timeline.

**Accepts when:** a rundown written by hand in a text editor loads, validates, and runs · load → save → load is byte-identical apart from formatting · every schema violation produces an error naming the line and the expectation · a `mode: at, guard: hard` item takes within one frame of its wall-clock time · a `guard: soft` item waits and logs the overrun · secondary events fire at their offsets within one frame, including negative offsets relative to item end · collision detection correctly flags an overrun 10 minutes in advance · a 200-item rundown runs unattended for two hours with no drift accumulation.

---

### Phase 10 — HTML source

Embedded HTML rendering with alpha, via CEF in a **separate helper process** (`rezie-html-helper`) per source, sharing frames with the engine over shared memory. Off-screen rendering with a transparent background. Reserved JS message channel (§12.1).

This is the largest single subsystem in the project. Budget accordingly. CEF must be a runtime-downloaded dependency, not committed, fetched by `xtask fetch-deps` and hash-verified.

**Accepts when:** a page with `background: transparent` renders with correct alpha over a video source · a CSS-animated page renders smoothly at the programme frame rate · killing the helper process leaves the engine running and shows the source as disconnected, then reconnects on retry · ten HTML sources run concurrently without exhausting the frame pool · memory is stable over a two-hour run with an animated page (leak test) · the helper is correctly sandboxed and cannot access the engine's memory.

---

### Phase 11 — Completion

M/E 2–4 enabled after performance measurement. Multiview. Project save/load with versioning and migration. Presets. Keyboard binding editor. Second-monitor docking. Settings. Performance instrumentation surfaced in the GUI. User documentation. Installers for all three platforms.

**Accepts when:** a project with 50 inputs, 4 M/Es, 10 overlays, and 4 outputs saves, loads, and reproduces state exactly · a project file from an earlier schema version migrates with a logged report · multiview with 16 tiles holds 12.5 fps without affecting programme timing · the GUI holds 30+ fps under that load · installers are signed where the platform requires it and install cleanly on a machine without a development toolchain · an eight-hour soak test at full configured load shows no memory growth beyond 2% and no dropped programme frames.

---

## 14. Testing

| Level | Scope | Where |
|---|---|---|
| Unit | Pure logic: scheduler timing, routing graph acyclicity, DSP coefficients, YAML round-trip | Alongside source |
| Golden frame | Every compositor path | `tests/golden/` |
| Integration | Engine driven through `rezie-api` over WebSocket; no GUI | `tests/integration/` |
| Soak | Long-running stability, memory, drift | `xtask soak`, nightly CI |
| Benchmark | Frame times, encode throughput, GPU/VRAM under defined loads | `xtask bench`, results committed to `docs/benchmarks/` per phase |

CI runs unit, golden, and integration on every commit across all three platforms. Soak and benchmark run nightly on the reference machine.

**A phase's benchmark numbers are committed.** A later phase that regresses frame time by more than 10% must justify it in an ADR or fix it.

---

## 15. Conventions

- `rustfmt` default, `clippy -D warnings`, both enforced in CI.
- Public items in every crate documented. `#![warn(missing_docs)]` on all library crates.
- No `unsafe` outside `rezie-ndi`, `rezie-media`, `rezie-capture`, and `rezie-html`, where FFI requires it. Every `unsafe` block carries a `// SAFETY:` comment stating the invariant.
- No `unwrap()` or `expect()` on any path reachable from user action. In tests, freely.
- Errors carry context. `"failed to open input"` is useless; `"failed to open input 'vt_mtb_report' (/media/vt/mtb.mp4): no video stream found"` is not.
- Commits are conventional-commit formatted and reference the phase: `feat(phase-4): overlay z-ordering`.
- Every phase gets a `docs/phases/NN-summary.md` recording what was built, what was deferred, what surprised you, and what the benchmarks said.

---

## 16. Open items requiring human decision

Raise these when the relevant phase is reached. Do not decide unilaterally.

1. **Application name and branding.** "HasaStudio" is a working codename. Confirm before phase 11 installers.
2. **HEVC software encoding.** §3.1 leaves the LGPL build without one. If software HEVC output turns out to be needed, the choice is between a GPL build (which forecloses closed-source distribution) and dropping HEVC on machines without hardware encode. Flag at phase 5.
3. **2160p50 output count.** VCN 3.0 will not sustain four 2160p50 HEVC encodes. Measure at phase 5 and agree the documented limit.
4. **Codec licensing.** H.264 and HEVC patent licensing (MPEG-LA / Access Advance) is a commercial question, not a technical one. Flag before any distribution.
5. **Project file format for binary-adjacent state.** §10 specifies YAML for rundowns. The project file is also YAML, which is right for hand-editing but will be large with 50 inputs. Revisit at phase 11 if load time exceeds 1 second.
