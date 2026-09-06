# 0032 — Use dav1d through FFmpeg for software AV1 decode

- **Status:** Proposed
- **Date:** 2026-09-06
- **Phase:** 1
- **Decided by:** Pending human review
- **Affects:** `rezie-media`, native dependency manifest, SPEC §2.1/§3.1

## Context

SPEC §13 includes AV1 file decode with software fallback in Phase 1, while
§2.1 says fallback uses “FFmpeg’s own LGPL decoders”. FFmpeg 7.1.1's native
`av1` decoder returns `AVERROR(ENOSYS)` when no hardware accelerator is
configured; it cannot supply that software fallback. H.264, HEVC and VP9
have native software decoders and do not require this exception.

Primary evidence: [FFmpeg 7.1.1 av1dec.c, lines 612–623](https://github.com/FFmpeg/FFmpeg/blob/n7.1.1/libavcodec/av1dec.c#L612-L623).
FFmpeg already provides a [libdav1d wrapper](https://github.com/FFmpeg/FFmpeg/blob/n7.1.1/libavcodec/libdav1d.c).

The exact Windows archive approved in ADR 0004 was downloaded for read-only
inspection and its SHA-256 verified. Its `avcodec-61.dll` contains an embedded
configuration with `--enable-libdav1d`, `--enable-shared`, `--disable-static`
and `--enable-version3`; `--enable-gpl` and `--enable-nonfree` are absent,
while x264/x265 are disabled. It contains the `libdav1d` decoder name and
ships LGPL v3 in `LICENSE.txt`. The [artifact inspection record](../testing/phase-1-ffmpeg-windows-artifact-audit.json) retains the exact configuration and hashes. This is artifact inspection, **not** execution
of the Windows library or proof that a file decodes. Only the aggregate
licence file is present; third-party notices still need to be assembled
before redistribution. ADR 0004's Windows pin remains unchanged.

The development Mac's installed FFmpeg is 9.0.1 with GPL, x264 and x265
enabled. It cannot be selected by pkg-config for this project. An isolated
FFmpeg 7.x LGPL shared build is needed on macOS/Linux.

## Options

### A — Select FFmpeg's libdav1d decoder for AV1 software fallback

Use the existing FFmpeg API, keeping all FFmpeg access in `rezie-media`.
dav1d is a native dependency under BSD-2-Clause. Preserve its copyright,
conditions and disclaimer in distributed notices. There is no direct Rust
binding to dav1d and no separate frame representation.

### B — Keep native av1 as the only fallback

Rejected: with hardware disabled, FFmpeg 7.1.1 returns ENOSYS. Labelling
this a fallback would leave a required codec unavailable.

### C — Use libaom instead

Also requires an external native decoder and the same clarification to
§2.1. Prefer dav1d because the already-approved Windows bundle includes it
and FFmpeg has a dedicated integration. No Rezie performance comparison of
these libraries has been measured or is claimed by this decision.

## Decision

**Proposed, awaiting approval:** select `libdav1d` for software AV1 decode;
retain FFmpeg's native H.264, HEVC and VP9 software decoders. Keep FFmpeg
dynamically linked and built without GPL/nonfree components. Hardware
selection and the explicit override that disables hardware remain required.

On approval, replace only the decode sentence in SPEC §2.1 with:

> Decode always has a software fallback through the LGPL FFmpeg build:
> FFmpeg's native decoders for H.264, HEVC and VP9, and its libdav1d
> integration (BSD-2-Clause) for AV1.

Add the corresponding dav1d licence/notice requirement to §3.1. Leave all
encoder policy and §16's commercial-review question unchanged.

For isolated macOS/Linux builds, pin dav1d **1.5.4**, the latest release in
VideoLAN's official release index when checked on 2026-09-06:

- URL: https://download.videolan.org/pub/videolan/dav1d/1.5.4/dav1d-1.5.4.tar.xz
- Size: 1,038,852 bytes.
- SHA-256: `686616b7c69eb88d44459391ab25cac13b6647a3b288835c5784e71c1514a5c5`.
- `COPYING` inspected inside that exact archive: BSD-2-Clause.
- Manifest consuming phase: `from_phase: 1`; source build isolated from
  system pkg-config paths, shared dav1d and shared FFmpeg.

The Windows bundle retains its existing artifact pin; do not pretend its
embedded dav1d is necessarily 1.5.4. Record the actual bundled version
when validating it on Windows. Changing that bundle remains a separate ADR.

## Consequences

AV1 software fallback becomes testable on every supported target through
the same demux/decode interface. The software override must select
`libdav1d` explicitly for AV1, not simply reopen native `av1` without a
hardware device. Hardware and software runs must record the selected
codec and actual hardware context/pixel format; a codec name alone does
not prove a hardware accelerator was used.

The native bootstrap must reject accidental selection of the Mac's system
FFmpeg and verify the exact libraries/configuration actually loaded.
Retain FFmpeg and dav1d notices and identify bundled versions; no encoder
or later-phase feature is introduced by this decoder decision.

This proposal changes no dependency manifest, links no native library and
amends no normative spec text until approved. Inspection archives are in
`/tmp`, outside the project dependency cache.

## Verification

Completed: inspected FFmpeg's pinned source, verified both archive hashes,
read dav1d's licence and the Windows bundle's embedded configuration.

After approval: build the isolated LGPL configuration; record library paths,
build configuration, versions and notices; decode a known AV1 fixture with
hardware disabled on all supported targets, assert `libdav1d`, and validate
frames, PTS and end-of-stream handling. Verify actual hardware decode
separately on equipped machines. No decoder execution result exists yet.

## Revisit when

FFmpeg gains native software AV1 decode, a security update requires a new
pin, the selected build lacks libdav1d, or packaging changes the licence
or linkage of the dependency.
