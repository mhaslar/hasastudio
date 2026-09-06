# 0033 — Bootstrap isolated FFmpeg and validate dedicated file decoders

- **Status:** Accepted
- **Date:** 2026-09-06
- **Phase:** 1
- **Decided by:** Agent implementation choices under approved ADR 0032
- **Affects:** rezie-media, engine startup, xtask, native setup and CI

## Context

Decode must use the approved FFmpeg/dav1d libraries, expose actual hardware
selection, retain presentation timestamps, and reject GPL/nonfree or ABI
mismatches at build and startup. Native setup must work before the engine
can be built; xtask currently imports the engine solely to read clock JSON.

## Options

### A — Isolated native prefix, direct library probes, pull-driven decoder

Build approved sources outside the repository source tree on macOS/Linux;
extract the already-pinned shared Windows bundle. Use pkg-config on Unix
and FFMPEG_DIR on Windows. Probe the selected native library at build time
and the process-linked library at engine startup. Decode on its owning
media worker (the diagnostic uses its main thread), never the clock thread.

### B — Use whatever FFmpeg is on PATH

Rejected: the development Mac's FFmpeg is GPL and ABI-incompatible. A
version printed by an unrelated executable is not loaded-library evidence.

## Decision

Implement A. Add spec-required ffmpeg-next =7.1.0 (WTFPL), selecting only
codec and format initially, with ffmpeg-sys-next locked to 7.1.3. Add
libloading =0.8.9 (ISC) and pkg-config =0.3.32 (MIT/Apache-2.0) for the
build probe. These Rust wrappers introduce no extra media SDK. The native
FFmpeg and dav1d additions are already approved in ADR 0032.

Use FFmpeg's official 7.1.1 source on Unix:
https://ffmpeg.org/releases/ffmpeg-7.1.1.tar.xz
SHA-256 `733984395e0dbbe5c046abda2dc49a5544e7e0e1e2366bba849222ae9e3a03b1`,
11,019,500 bytes. It was fetched and hashed before implementation.
Use dav1d 1.5.4 and its hash from ADR 0032. Manifest eligibility gains a
Unix selector; both sources consume Phase 1 only. No source is vendored.

Bootstrap shared FFmpeg/dav1d in `.deps/native`; do not change system
installations. Enable Phase 1 video decoders/demuxers and platform hardware
acceleration only; no project encoder feature or later-phase ingest is
introduced. Linux uses the spec-required VAAPI interface (libva development
headers, MIT); macOS uses system VideoToolbox frameworks; Windows uses the
bundle's D3D11VA implementation. Meson 1.9.1 / Ninja 1.13.0 are isolated pinned build tools,
not application dependencies. Build preparation is explicit, not a network
side effect of a Rust build script. Native CI caches are keyed by the native
manifest and bootstrap script.

Relocate the development package’s shared libraries with install_name_tool/ad-hoc codesigning on macOS and patchelf (build tool only) on Linux; copy the pinned DLLs beside Windows executables. Launch smoke tests with loader override variables removed. Commercial distribution remains subject to SPEC §16 item 4.

Use libavcodec's reported major 61 and configuration/licence at both guards.
Build-time library selection must match FFmpeg bindings' selection; unsupported
cross compilation fails explicitly because a host probe cannot validate a
foreign target library. Native builds on the three targets remain supported.

The decoder exposes borrowed codec staging planes and integer PTS/time base;
it does not introduce an owned CPU programme-frame type. Downstream publication
remains through FramePool working textures after ingest. Preserve colour
metadata and alpha information; no implicit scaling, frame-rate conversion or
colour transform in decode. Reuse decoder frames; drain delayed output at EOF.
An explicit hardware-disable override selects native H.264/HEVC/VP9 or
libdav1d for AV1. Automatic hardware failure reopens software decode and
replays past already-delivered frames before resuming, avoiding duplicates;
report the fallback reason. Strict hardware diagnostics fail instead.

Remove xtask's engine dependency to break the bootstrap cycle. Its sweep
reader uses a private serde projection of the existing report plus the
existing rezie-rt report types; serialized clock evidence is unchanged.
The engine itself has no licence-check bypass, feature flag or warning mode.

On macOS, query the actual Apple session hardware property through the
optional accessor documented in ADR 0034. An AVHWFramesContext alone cannot
prove HEVC silicon decode, and application code must not read FFmpeg private
layouts. The approved Windows archive remains unchanged.

## Consequences

Clean builds need native preparation. Missing native libraries fail with the
exact setup instruction. Binary distribution must carry the shared libraries
and notices; running a build-tree binary is not evidence of a relocatable
package. Changes to packaging are tested before declaring this slice ready.

Hardware correctness needs equipped machines. Portable CI proves software
decode and both policy guards; missing hardware never counts as hardware
success. GPU upload/preview integration and NDI remain subsequent Phase 1
work after the decoder is validated, as approved by the owner.

Small committed compressed fixtures contain a deterministic pattern owned by
this project. A standalone external FFmpeg authoring executable creates the
bitstreams once (including H.264 B-frames) and independently records raw
sample hashes and ffprobe PTS. Its exact configuration is in the fixture
manifest. That authoring tool may contain GPL encoders; no code/library from
it is linked, copied or packaged with Rezie, and normal builds/tests never
invoke it. Application and test decode always use the approved LGPL build.
The compressed test files, not the authoring executable, are committed.

## Verification

Assert real loaded configurations/versions on all three CI platforms. Compile
small incompatible native fixtures to test build rejection and the shared
startup guard; check GPL, nonfree and wrong-major failures separately. Decode
fixture files to EOF, verify frame counts, per-frame PTS and sample digests,
exercise hardware-disable, malformed input and delayed frames. Record actual
hardware device/context and decoded hardware pixel format on equipped hosts.
No hardware or decode execution result is claimed by this ADR alone.

## Revisit when

The FFmpeg major/pins change, new native components are required, hardware
transfer becomes a measured bottleneck, or cross compilation is needed.
